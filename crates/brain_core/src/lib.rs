mod thought;
mod pipeline;
mod llm;
mod config;

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use log::{info, error, warn};
use std::path::Path;

use memory::{MemorySystem, Goal, GoalStatus};
use thought::ThoughtCloud;
use pipeline::Pipeline;
use config::BrainConfig;

/// The main Brain structure that coordinates all components
pub struct Brain {
    memory_system: Arc<Mutex<MemorySystem>>,
    thought_cloud: Arc<Mutex<ThoughtCloud>>,
    pipeline: Pipeline,
    running: Arc<Mutex<bool>>,
    config: BrainConfig,
}

impl Brain {
    /// Create a new Brain instance
    pub async fn new() -> Result<Self> {
        // Load default config
        Self::with_config(BrainConfig::default()).await
    }

    /// Create a new Brain instance with a specific config file
    pub async fn with_config_file<P: AsRef<Path>>(config_path: P) -> Result<Self> {
        let config = BrainConfig::load_or_default(config_path);
        Self::with_config(config).await
    }
    
    /// Create a new Brain instance with the given config
    pub async fn with_config(config: BrainConfig) -> Result<Self> {
        // Log brain configuration information
        info!("Initializing The Brain with the following configuration:");
        info!("- Version: 0.1.0");
        
        // Get environment information
        let env_vars = vec![
            ("ANTHROPIC_API_KEY", std::env::var("ANTHROPIC_API_KEY").map(|v| if v.is_empty() { "Not set".to_string() } else { "Set (hidden)".to_string() }).unwrap_or_else(|_| "Not set".to_string())),
            ("RUST_LOG", std::env::var("RUST_LOG").unwrap_or_else(|_| "Not set".to_string())),
        ];
        
        for (name, value) in env_vars {
            info!("- Environment: {} = {}", name, value);
        }
        
        // Initialize memory system
        info!("Initializing memory system...");
        let memory_system = Arc::new(Mutex::new(MemorySystem::new().await?));
        
        // Add some initial goals
        let initial_goals = vec![
            Goal {
                id: uuid::Uuid::new_v4().to_string(),
                description: "Understand current user's immediate needs".to_string(),
                priority: 10,
                status: GoalStatus::Active,
                created_at: chrono::Utc::now(),
                completed_at: None,
                related_thoughts: Vec::new(),
                progress: 0.0,
                parent_goal: None,
                sub_goals: Vec::new(),
            },
            Goal {
                id: uuid::Uuid::new_v4().to_string(),
                description: "Build context from user interactions".to_string(),
                priority: 8,
                status: GoalStatus::Active,
                created_at: chrono::Utc::now(),
                completed_at: None,
                related_thoughts: Vec::new(),
                progress: 0.0,
                parent_goal: None,
                sub_goals: Vec::new(),
            },
            Goal {
                id: uuid::Uuid::new_v4().to_string(),
                description: "Identify patterns in user requests".to_string(),
                priority: 7,
                status: GoalStatus::Active,
                created_at: chrono::Utc::now(),
                completed_at: None,
                related_thoughts: Vec::new(),
                progress: 0.0,
                parent_goal: None,
                sub_goals: Vec::new(),
            },
            Goal {
                id: uuid::Uuid::new_v4().to_string(),
                description: "Develop personalized assistance strategies".to_string(),
                priority: 6,
                status: GoalStatus::Active,
                created_at: chrono::Utc::now(),
                completed_at: None,
                related_thoughts: Vec::new(),
                progress: 0.0,
                parent_goal: None,
                sub_goals: Vec::new(),
            }
        ];

        for goal in initial_goals {
            let mut memory = memory_system.lock().await;
            memory.add_goal(goal).await?;
        }
        
        // Initialize thought cloud with capacity from config
        info!("Initializing thought cloud...");
        let thought_cloud_capacity = config.core.thought_cloud_capacity;
        let thought_cloud = Arc::new(Mutex::new(ThoughtCloud::with_capacity(thought_cloud_capacity)));
        info!("- Thought cloud capacity: {}", thought_cloud_capacity);
        
        // Initialize pipeline with values from config
        info!("Initializing thought pipeline...");
        let pipeline = Pipeline::new_with_config(
            memory_system.clone(), 
            thought_cloud.clone(),
            &config
        );
        
        // Log pipeline configuration
        info!("- Pipeline stages: ThoughtGeneration -> BaseInstinct -> ValuesAssessment -> PsychologicalProcessing -> ExternalizationDecision");
        info!("- Core values: {}", config.values.core_values.join(", "));
        info!("- Externalization threshold: {}", config.pipeline.externalization_threshold);
        
        // Log parallelism configuration
        let parallelism = &config.pipeline.parallelism;
        info!("- Parallel processing: {}", if parallelism.enabled { "Enabled" } else { "Disabled" });
        if parallelism.enabled {
            info!("  * Thought generation: {} threads", parallelism.thought_generation);
            info!("  * Base instinct: {} threads", parallelism.base_instinct);
            info!("  * Values assessment: {} threads", parallelism.values_assessment);
            info!("  * Psychological processing: {} threads", parallelism.psychological_processing);
            info!("  * Externalization decision: {} threads", parallelism.externalization_decision);
        }
        
        // Log psychological structures
        use crate::pipeline::PsychologicalStructure;
        let structures = PsychologicalStructure::all();
        info!("- Psychological structures: {}", structures.iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", "));
            
        Ok(Self {
            memory_system,
            thought_cloud,
            pipeline,
            running: Arc::new(Mutex::new(false)),
            config,
        })
    }
    
    /// Start the Brain's continuous thought process
    pub async fn start(&self) -> Result<()> {
        info!("Starting Brain's continuous thought process");
        
        // Log thought processing configuration
        info!("Thought Processing Configuration:");
        info!("- Process delay: 5 seconds per thought cycle");
        info!("- Anti-stagnation: Enabled (will skip stages after 3+ cycles)");
        info!("- Thought diversity: Enabled (1 in 8 chance of new thoughts)");
        info!("- Goal-directed thinking: Enabled (goals reviewed every 50 cycles)");
        
        // Log current thought cloud status
        let thought_count = {
            let cloud = self.thought_cloud.lock().await;
            cloud.len()
        };
        info!("- Current thought cloud size: {}", thought_count);
        
        // Log memory system status
        let memory_stats = {
            let memory = self.memory_system.lock().await;
            (
                memory.get_recent_thoughts(0).await.map(|t| t.len()).unwrap_or(0),
                memory.get_important_memories(0).await.map(|m| m.len()).unwrap_or(0),
                memory.get_recent_tool_executions(0).await.map(|e| e.len()).unwrap_or(0),
                memory.get_active_goals().await.map(|g| g.len()).unwrap_or(0)
            )
        };
        
        info!("- Memory system status: {} thoughts, {} memories, {} tool executions, {} active goals", 
              memory_stats.0, memory_stats.1, memory_stats.2, memory_stats.3);
        
        // Set running flag to true
        {
            let mut running = self.running.lock().await;
            *running = true;
        }
        
        // Clone Arc references for the thought process task
        let pipeline = self.pipeline.clone();
        let running = self.running.clone();
        let config_clone = self.config.clone(); // Clone the config for the task
        
        // Spawn the continuous thought process
        tokio::spawn(async move {
            info!("Thought process background task started");
            
            let mut cycle_count = 0;
            let start_time = std::time::Instant::now();
            let is_parallel = config_clone.pipeline.parallelism.enabled;
            let delay_seconds = config_clone.core.thought_cycle_delay;
            
            // Define goal review frequency
            let goal_review_cycle = 50; // Review goals every 50 cycles
            
            while *running.lock().await {
                cycle_count += 1;
                
                // Run one cycle of the thought pipeline
                match pipeline.process_next_thought().await {
                    Ok(_) => {
                        // Successfully processed thought(s)
                        if cycle_count % 10 == 0 {
                            let elapsed = start_time.elapsed();
                            
                            if is_parallel {
                                info!("Thought cycle {} completed (parallel processing, {:.2} cycles/minute)", 
                                    cycle_count, 
                                    (cycle_count as f64) / (elapsed.as_secs() as f64 / 60.0));
                            } else {
                                info!("Thought cycle {} completed ({:.2} cycles/minute)", 
                                    cycle_count, 
                                    (cycle_count as f64) / (elapsed.as_secs() as f64 / 60.0));
                            }
                        }
                        
                        // Periodically review and update goals
                        if cycle_count % goal_review_cycle == 0 {
                            info!("Reviewing and updating goals (cycle {})", cycle_count);
                            if let Err(e) = pipeline.generate_or_update_goals().await {
                                warn!("Error reviewing goals: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Error processing thought: {}", e);
                    }
                }
                
                // Delay to slow down the thought process based on config
                tokio::time::sleep(tokio::time::Duration::from_secs(delay_seconds)).await;
            }
            
            info!("Thought process background task stopped after {} cycles", cycle_count);
        });
        
        info!("Brain's thought process started");
        Ok(())
    }
    
    /// Stop the Brain's thought process
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping Brain's thought process");
        
        // Set running flag to false to stop the thought process
        let mut running = self.running.lock().await;
        *running = false;
        
        info!("Brain's thought process stopped");
        Ok(())
    }
    
    /// Get a reference to the memory system
    pub fn memory_system(&self) -> Arc<Mutex<MemorySystem>> {
        self.memory_system.clone()
    }
    
    /// Get a reference to the thought cloud
    pub fn thought_cloud(&self) -> Arc<Mutex<ThoughtCloud>> {
        self.thought_cloud.clone()
    }
    
    /// Send a message from the user to the brain
    pub async fn send_user_message(&self, message: &str) -> Result<()> {
        // Store the message in memory
        let mut memory = self.memory_system.lock().await;
        memory.store_chat_message(memory::ChatSender::User, message).await?;
        
        // Create a thought based on this message for processing
        let mut thought = thought::Thought::new(&format!("User message: {}", message));
        thought.thought_type = "user_message".to_string();
        
        // Add it to the thought cloud for processing
        let mut cloud = self.thought_cloud.lock().await;
        cloud.add_thought(thought);
        
        info!("Added user message to thought cloud: {}", message);
        
        Ok(())
    }
    
    /// Get recent chat messages
    pub async fn get_chat_messages(&self, limit: usize) -> Result<Vec<memory::ChatMessage>> {
        let memory = self.memory_system.lock().await;
        memory.get_recent_chat_messages(limit).await
    }
    
    /// Get all active goals
    pub async fn get_active_goals(&self) -> Result<Vec<Goal>> {
        let memory = self.memory_system.lock().await;
        memory.get_active_goals().await
    }
    
    /// Add a new goal with the given description and priority
    pub async fn add_goal(&self, description: &str, priority: u8) -> Result<String> {
        let goal = Goal {
            id: uuid::Uuid::new_v4().to_string(),
            description: description.to_string(),
            priority: priority.clamp(1, 10),
            status: GoalStatus::Active,
            created_at: chrono::Utc::now(),
            completed_at: None,
            related_thoughts: Vec::new(),
            progress: 0.0,
            parent_goal: None,
            sub_goals: Vec::new(),
        };
        
        let mut memory = self.memory_system.lock().await;
        memory.add_goal(goal).await
    }
    
    /// Update a goal's status
    pub async fn update_goal_status(&self, goal_id: &str, status: GoalStatus) -> Result<()> {
        let mut memory = self.memory_system.lock().await;
        memory.update_goal_status(goal_id, status).await
    }
    
    /// Update a goal's progress
    pub async fn update_goal_progress(&self, goal_id: &str, progress: f32) -> Result<()> {
        let mut memory = self.memory_system.lock().await;
        memory.update_goal_progress(goal_id, progress).await
    }
    
    /// Set a new high-priority goal for the brain and ensure it steers thoughts towards it
    pub async fn set_priority_goal(&self, description: &str) -> Result<String> {
        // Create a high-priority goal (priority 10)
        self.add_goal(description, 10).await
    }
    
    /// Trigger a goal reassessment
    pub async fn reassess_goals(&self) -> Result<()> {
        self.pipeline.generate_or_update_goals().await
    }

    /// Get progress on a specific goal by description (fuzzy match)
    pub async fn get_goal_progress(&self, description_keywords: &str) -> Result<Option<f32>> {
        let memory = self.memory_system.lock().await;
        let active_goals = memory.get_active_goals().await?;
        
        // Find goal with matching description (case-insensitive partial match)
        let keywords = description_keywords.to_lowercase();
        
        let matching_goal = active_goals.iter()
            .find(|g| g.description.to_lowercase().contains(&keywords));
            
        if let Some(goal) = matching_goal {
            Ok(Some(goal.progress))
        } else {
            Ok(None)
        }
    }
}