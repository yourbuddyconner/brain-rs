use std::sync::Arc;
use std::any::Any;
use tokio::sync::Mutex;
use color_eyre::Result;
use ratatui::{
    prelude::*,
    widgets::*,
    style::{Color, Modifier, Style},
    layout::{Constraint, Direction, Layout},
    text::{Line, Span, Text},
};
use tokio::sync::mpsc::UnboundedSender;
use brain_core::Brain;
use memory::{GoalStatus, ChatSender};

use super::Component;
use crate::{action::Action, config::Config};

pub struct Home {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    brain: Option<Arc<Brain>>,
    input_text: String,
    running_status: bool,
    current_tab: usize,
    scroll_positions: [u16; 4], // Store scroll positions for different panels
    chat_messages: Vec<(String, String)>,
    thoughts_data: Vec<(String, String)>,
    goals_data: Vec<(String, u8, f32, GoalStatus)>,
    stats_data: Vec<(String, String)>,
    last_update: std::time::Instant,
}

impl Default for Home {
    fn default() -> Self {
        Self {
            command_tx: None,
            config: Config::default(),
            brain: None,
            input_text: String::new(),
            running_status: false,
            current_tab: 0,
            scroll_positions: [0, 0, 0, 0],
            chat_messages: Vec::new(),
            thoughts_data: Vec::new(),
            goals_data: Vec::new(),
            stats_data: Vec::new(),
            last_update: std::time::Instant::now(),
        }
    }
}

impl Home {
    pub fn new() -> Self {
        Self {
            command_tx: None,
            config: Config::default(),
            brain: None,
            input_text: String::new(),
            running_status: false,
            current_tab: 0,
            scroll_positions: [0, 0, 0, 0],
            chat_messages: Vec::new(),
            thoughts_data: Vec::new(),
            goals_data: Vec::new(),
            stats_data: Vec::new(),
            last_update: std::time::Instant::now(),
        }
    }

    pub fn set_brain(&mut self, brain: Arc<Brain>) {
        self.brain = Some(brain);
    }

    fn handle_input(&mut self, c: char) -> Result<Option<Action>> {
        self.input_text.push(c);
        Ok(None)
    }

    fn handle_backspace(&mut self) -> Result<Option<Action>> {
        self.input_text.pop();
        Ok(None)
    }

    fn submit_message(&mut self) -> Result<Option<Action>> {
        if !self.input_text.is_empty() {
            if let Some(brain) = &self.brain {
                let message = self.input_text.clone();
                self.input_text.clear();
                
                // Add message to chat messages immediately for better UI responsiveness
                self.chat_messages.push(("User".to_string(), message.clone()));
                
                // Clone the brain reference for async block
                let brain_clone = brain.clone();
                
                // Send in a separate task to avoid blocking
                if let Some(tx) = &self.command_tx {
                    let tx_clone = tx.clone();
                    tokio::spawn(async move {
                        if let Err(e) = brain_clone.send_user_message(&message).await {
                            tracing::error!("Failed to send message: {}", e);
                        } else {
                            // Force a refresh of the data after sending a message
                            let _ = tx_clone.send(Action::Help);
                        }
                    });
                }
            }
        }
        Ok(None)
    }

    fn start_stop_brain(&mut self) -> Result<Option<Action>> {
        if let Some(brain) = &self.brain {
            let brain_clone = brain.clone();
            
            if self.running_status {
                // Stop the brain
                tokio::spawn(async move {
                    if let Err(e) = brain_clone.stop().await {
                        tracing::error!("Failed to stop brain: {}", e);
                    }
                });
                self.running_status = false;
                
                // Update the status immediately in the stats data
                for item in self.stats_data.iter_mut() {
                    if item.0 == "Status" {
                        item.1 = "Stopped".to_string();
                        break;
                    }
                }
                
                // If status isn't already in stats_data, add it
                if !self.stats_data.iter().any(|(key, _)| key == "Status") {
                    self.stats_data.push(("Status".to_string(), "Stopped".to_string()));
                }
                
                tracing::info!("Brain stopped");
            } else {
                // Start the brain
                tokio::spawn(async move {
                    if let Err(e) = brain_clone.start().await {
                        tracing::error!("Failed to start brain: {}", e);
                    }
                });
                self.running_status = true;
                
                // Update the status immediately in the stats data
                for item in self.stats_data.iter_mut() {
                    if item.0 == "Status" {
                        item.1 = "Running".to_string();
                        break;
                    }
                }
                
                // If status isn't already in stats_data, add it
                if !self.stats_data.iter().any(|(key, _)| key == "Status") {
                    self.stats_data.push(("Status".to_string(), "Running".to_string()));
                }
                
                tracing::info!("Brain started");
            }
            
            // Force an immediate data refresh
            if let Some(tx) = &self.command_tx {
                tx.send(Action::Help)?;
            }
        }
        Ok(None)
    }

    fn change_tab(&mut self, tab: usize) -> Result<Option<Action>> {
        self.current_tab = tab;
        Ok(None)
    }

    fn scroll_up(&mut self) -> Result<Option<Action>> {
        if self.scroll_positions[self.current_tab] > 0 {
            self.scroll_positions[self.current_tab] -= 1;
        }
        Ok(None)
    }

    fn scroll_down(&mut self) -> Result<Option<Action>> {
        self.scroll_positions[self.current_tab] += 1;
        Ok(None)
    }
}

impl Component for Home {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.config = config;
        Ok(())
    }

    fn handle_key_event(&mut self, key: crossterm::event::KeyEvent) -> Result<Option<Action>> {
        match key.code {
            crossterm::event::KeyCode::Char(c) => self.handle_input(c),
            crossterm::event::KeyCode::Backspace => self.handle_backspace(),
            crossterm::event::KeyCode::Enter => self.submit_message(),
            crossterm::event::KeyCode::Tab => self.change_tab((self.current_tab + 1) % 4),
            crossterm::event::KeyCode::F(1) => self.start_stop_brain(),
            crossterm::event::KeyCode::Up => self.scroll_up(),
            crossterm::event::KeyCode::Down => self.scroll_down(),
            _ => Ok(None),
        }
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => {
                // Update cached data on tick
                self.update_cached_data()?;
            }
            Action::Render => {
                // Render-specific logic
            }
            Action::Help => {
                // This is our signal for data updates from async tasks
                if let Some(brain) = &self.brain {
                    // Create a separate thread to update the local cache
                    // We'll execute these synchronously since we're in a Tick handler
                    let brain_clone = brain.clone();
                    
                    // Update chat messages
                    match futures::executor::block_on(brain_clone.get_chat_messages(20)) {
                        Ok(messages) => {
                            self.chat_messages = messages.into_iter()
                                .map(|msg| {
                                    let sender = match msg.sender {
                                        ChatSender::Brain => "Brain".to_string(),
                                        ChatSender::User => "User".to_string(),
                                    };
                                    (sender, msg.content)
                                })
                                .collect();
                        }
                        Err(e) => {
                            tracing::error!("Failed to fetch chat messages: {}", e);
                        }
                    }
                    
                    // Update thoughts
                    let thought_cloud = brain_clone.thought_cloud();
                    let cloud = futures::executor::block_on(async {
                        thought_cloud.lock().await
                    });
                    
                    let thoughts = cloud.list_thoughts();
                    self.thoughts_data = thoughts.into_iter()
                        .map(|t| (t.pipeline_stage, t.content))
                        .collect();
                    
                    // Update goals
                    match futures::executor::block_on(brain_clone.get_active_goals()) {
                        Ok(goals) => {
                            self.goals_data = goals.into_iter()
                                .map(|g| (g.description, g.priority, g.progress, g.status))
                                .collect();
                        }
                        Err(e) => {
                            tracing::error!("Failed to fetch goals: {}", e);
                        }
                    }
                    
                    // Update stats
                    self.stats_data.clear();
                    
                    // Status
                    self.stats_data.push((
                        "Status".to_string(), 
                        if self.running_status { "Running".to_string() } else { "Stopped".to_string() }
                    ));
                    
                    // Thought cloud size
                    let thought_cloud = brain_clone.thought_cloud();
                    let cloud = futures::executor::block_on(async {
                        thought_cloud.lock().await
                    });
                    self.stats_data.push(("Thought Cloud Size".to_string(), format!("{} thoughts", cloud.len())));
                    
                    // Active goals count
                    if let Ok(goals) = futures::executor::block_on(brain_clone.get_active_goals()) {
                        self.stats_data.push(("Active Goals".to_string(), format!("{} goals", goals.len())));
                    }
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        // Create the layout
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Title bar
                Constraint::Min(5),     // Main content
                Constraint::Length(3),  // Input area
            ])
            .split(area);

        // Render title bar
        let title_text = if let Some(_brain) = &self.brain {
            let status = if self.running_status { 
                "Running" 
            } else { 
                "Stopped" 
            };
            format!("The Brain | Status: {} | Press F1 to Start/Stop | Tab to switch views", status)
        } else {
            "The Brain | Not initialized".to_string()
        };

        let title = Paragraph::new(title_text)
            .style(Style::default().fg(Color::White).bg(Color::Blue))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        
        frame.render_widget(title, main_layout[0]);

        // Tab bar
        let tabs = Tabs::new(vec!["Chat", "Thoughts", "Goals", "Status"])
            .select(self.current_tab)
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL));
        
        let tab_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Tab bar
                Constraint::Min(3),     // Tab content
            ])
            .split(main_layout[1]);
        
        frame.render_widget(tabs, tab_area[0]);

        // Render different content based on selected tab
        match self.current_tab {
            0 => self.draw_chat_tab(frame, tab_area[1])?,
            1 => self.draw_thoughts_tab(frame, tab_area[1])?,
            2 => self.draw_goals_tab(frame, tab_area[1])?,
            _ => self.draw_status_tab(frame, tab_area[1])?,
        }

        // Input area
        let input = Paragraph::new(self.input_text.clone())
            .style(Style::default().fg(Color::White))
            .block(Block::default().borders(Borders::ALL).title("Enter Message"));
        
        frame.render_widget(input, main_layout[2]);

        Ok(())
    }
    
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}

impl Home {
    
    fn update_cached_data(&mut self) -> Result<()> {
        // Only update every 500ms to avoid excessive tokio task spawning
        let now = std::time::Instant::now();
        if now.duration_since(self.last_update).as_millis() < 500 {
            return Ok(());
        }
        
        self.last_update = now;
        
        if let Some(brain) = &self.brain {
            let brain_clone = brain.clone();
            
            // Update chat messages
            if let Some(tx) = &self.command_tx {
                let tx_clone = tx.clone();
                tokio::spawn(async move {
                    match brain_clone.get_chat_messages(20).await {
                        Ok(messages) => {
                            let _formatted_messages = messages.into_iter()
                                .map(|msg| {
                                    let sender = match msg.sender {
                                        ChatSender::Brain => "Brain".to_string(),
                                        ChatSender::User => "User".to_string(),
                                    };
                                    (sender, msg.content)
                                })
                                .collect::<Vec<_>>();
                            
                            // Use a custom action to update the chat messages
                            let _ = tx_clone.send(Action::Help); // Using Help as a signal for updates
                        }
                        Err(e) => {
                            tracing::error!("Failed to fetch chat messages: {}", e);
                        }
                    }
                });
            }
            
            // Update thoughts
            if let Some(tx) = &self.command_tx {
                let tx_clone = tx.clone();
                let brain_clone = brain.clone();
                tokio::spawn(async move {
                    // Get the lock on the thought cloud
                    let thought_cloud = brain_clone.thought_cloud();
                    let cloud = thought_cloud.lock().await;
                    let thoughts = cloud.list_thoughts();
                    let thoughts_data = thoughts.into_iter()
                        .map(|t| (t.pipeline_stage, t.content))
                        .collect::<Vec<_>>();
                    
                    // Use a custom action to update the thoughts
                    let _ = tx_clone.send(Action::Help); // Using Help as a signal for updates
                });
            }
            
            // Update goals
            if let Some(tx) = &self.command_tx {
                let tx_clone = tx.clone();
                let brain_clone = brain.clone();
                tokio::spawn(async move {
                    match brain_clone.get_active_goals().await {
                        Ok(goals) => {
                            let _formatted_goals = goals.into_iter()
                                .map(|g| (g.description, g.priority, g.progress, g.status))
                                .collect::<Vec<_>>();
                            
                            // Use a custom action to update the goals
                            let _ = tx_clone.send(Action::Help); // Using Help as a signal for updates
                        }
                        Err(e) => {
                            tracing::error!("Failed to fetch goals: {}", e);
                        }
                    }
                });
            }
            
            // Update stats
            if let Some(tx) = &self.command_tx {
                let tx_clone = tx.clone();
                let brain_clone = brain.clone();
                tokio::spawn(async move {
                    let mut stats = Vec::new();
                    
                    // Status
                    stats.push(("Status".to_string(), "Running".to_string())); // This will be updated by the TUI
                    
                    // Thought cloud size
                    let thought_cloud = brain_clone.thought_cloud();
                    let cloud = thought_cloud.lock().await;
                    stats.push(("Thought Cloud Size".to_string(), format!("{} thoughts", cloud.len())));
                    
                    // Active goals
                    if let Ok(goals) = brain_clone.get_active_goals().await {
                        stats.push(("Active Goals".to_string(), format!("{} goals", goals.len())));
                    }
                    
                    // Use a custom action to update the stats
                    let _ = tx_clone.send(Action::Help); // Using Help as a signal for updates
                });
            }
        }
        
        Ok(())
    }

    fn draw_chat_tab(&self, frame: &mut Frame, area: Rect) -> Result<()> {
        // Create initial messages
        let initial_messages = vec![
            ("Brain".to_string(), "Welcome to The Brain. How can I assist you today?".to_string())
        ];
        
        let not_initialized = vec![
            ("System".to_string(), "Brain not initialized".to_string())
        ];
        
        let messages = if !self.chat_messages.is_empty() {
            &self.chat_messages
        } else if let Some(_) = &self.brain {
            &initial_messages
        } else {
            &not_initialized
        };

        let messages_styled: Vec<ListItem> = messages
            .iter()
            .map(|(sender, content)| {
                let (style, prefix) = match sender.as_str() {
                    "Brain" => (Style::default().fg(Color::Blue), "🧠 "),
                    "User" => (Style::default().fg(Color::Green), "👤 "),
                    _ => (Style::default().fg(Color::Gray), "🔄 "),
                };
                
                let line = Line::from(vec![
                    Span::styled(format!("{}{}: ", prefix, sender), style.add_modifier(Modifier::BOLD)),
                    Span::raw(content),
                ]);
                
                ListItem::new(line)
            })
            .collect();

        let messages_list = List::new(messages_styled)
            .block(Block::default().borders(Borders::ALL).title("Chat History"))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        frame.render_widget(messages_list, area);
        
        Ok(())
    }

    fn draw_thoughts_tab(&self, frame: &mut Frame, area: Rect) -> Result<()> {
        // Initial thoughts
        let initial_thoughts = vec![
            ("base_instinct".to_string(), "I should analyze what the user needs".to_string()),
            ("values_assessment".to_string(), "This request aligns with helping the user".to_string()),
            ("psychological_processing".to_string(), "User might be interested in X based on their query".to_string()),
        ];
        
        let not_initialized = vec![
            ("System".to_string(), "Brain not initialized".to_string()),
        ];
        
        let thoughts = if !self.thoughts_data.is_empty() {
            &self.thoughts_data
        } else if let Some(_) = &self.brain {
            &initial_thoughts
        } else {
            &not_initialized
        };

        let thoughts_styled: Vec<ListItem> = thoughts
            .iter()
            .map(|(stage, content)| {
                let style = match stage.as_str() {
                    "base_instinct" => Style::default().fg(Color::Red),
                    "values_assessment" => Style::default().fg(Color::Yellow),
                    "psychological_processing" => Style::default().fg(Color::Magenta),
                    "externalization_decision" => Style::default().fg(Color::Cyan),
                    _ => Style::default().fg(Color::Gray),
                };
                
                let stage_name = match stage.as_str() {
                    "base_instinct" => "Base Instinct",
                    "values_assessment" => "Values Assessment",
                    "psychological_processing" => "Psychological",
                    "externalization_decision" => "Externalization",
                    _ => stage.as_str(),
                };
                
                let line = Line::from(vec![
                    Span::styled(format!("[{}] ", stage_name), style.add_modifier(Modifier::BOLD)),
                    Span::raw(content),
                ]);
                
                ListItem::new(line)
            })
            .collect();

        let thoughts_list = List::new(thoughts_styled)
            .block(Block::default().borders(Borders::ALL).title("Thought Cloud"))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        frame.render_widget(thoughts_list, area);
        
        Ok(())
    }

    fn draw_goals_tab(&self, frame: &mut Frame, area: Rect) -> Result<()> {
        // Initial goals
        let initial_goals = vec![
            ("Understand user's immediate needs".to_string(), 10, 0.7, GoalStatus::Active),
            ("Build context from interactions".to_string(), 8, 0.5, GoalStatus::Active),
            ("Identify patterns in requests".to_string(), 7, 0.3, GoalStatus::Active),
            ("Develop assistance strategies".to_string(), 6, 0.1, GoalStatus::Active),
        ];
        
        let not_initialized = vec![
            ("Brain not initialized".to_string(), 0, 0.0, GoalStatus::Blocked),
        ];
        
        let goals = if !self.goals_data.is_empty() {
            &self.goals_data
        } else if let Some(_) = &self.brain {
            &initial_goals
        } else {
            &not_initialized
        };

        let goals_styled: Vec<ListItem> = goals
            .iter()
            .map(|(desc, priority, progress, status)| {
                let status_style = match status {
                    GoalStatus::Active => Style::default().fg(Color::Green),
                    GoalStatus::Completed => Style::default().fg(Color::Blue),
                    GoalStatus::Abandoned => Style::default().fg(Color::Red),
                    GoalStatus::Blocked => Style::default().fg(Color::Yellow),
                };
                
                let progress_percent = (*progress * 100.0) as u16;
                // Create a progress indicator in text instead of using gauge widget
                let progress_indicator = format!(" [{}%]", progress_percent);
                
                let mut lines = vec![
                    Line::from(vec![
                        Span::styled(format!("[P{}] ", priority), Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(desc),
                        Span::styled(progress_indicator, status_style),
                    ])
                ];
                
                ListItem::new(lines)
            })
            .collect();

        let goals_list = List::new(goals_styled)
            .block(Block::default().borders(Borders::ALL).title("Active Goals"))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        frame.render_widget(goals_list, area);
        
        Ok(())
    }

    fn draw_status_tab(&self, frame: &mut Frame, area: Rect) -> Result<()> {
        // Initial stats
        let initial_stats = vec![
            ("Status".to_string(), "Initializing...".to_string()),
            ("Thought Cloud Size".to_string(), "Loading...".to_string()),
            ("Active Goals".to_string(), "Loading...".to_string()),
        ];
        
        let not_initialized = vec![
            ("Status".to_string(), "Brain not initialized".to_string()),
        ];
        
        let stats = if !self.stats_data.is_empty() {
            &self.stats_data
        } else if let Some(_) = &self.brain {
            &initial_stats
        } else {
            &not_initialized
        };

        let status_text = stats
            .iter()
            .map(|(label, value)| {
                format!("{}: {}", label, value)
            })
            .collect::<Vec<String>>()
            .join("\n");

        let status_paragraph = Paragraph::new(status_text)
            .block(Block::default().borders(Borders::ALL).title("System Status"))
            .alignment(Alignment::Left);

        frame.render_widget(status_paragraph, area);
        
        Ok(())
    }
}
