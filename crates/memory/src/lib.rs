use anyhow::Result;
use uuid::Uuid;
use chrono::{self, DateTime, Utc};
use serde::{Serialize, Deserialize};
use serde_json::Value;

/// Represents a memory system for the brain
pub struct MemorySystem {
    // In a real implementation, this would hold a database connection
    // For MVP, we'll use in-memory storage
    thoughts: Vec<Thought>,
    memories: Vec<Memory>,
    tool_executions: Vec<ToolExecution>,
    chat_messages: Vec<ChatMessage>,
    goals: Vec<Goal>,
}

impl MemorySystem {
    // Goal management methods

    /// Add a new goal to the memory system
    pub async fn add_goal(&mut self, goal: Goal) -> Result<String> {
        self.goals.push(goal.clone());
        Ok(goal.id)
    }

    /// Get all active goals
    pub async fn get_active_goals(&self) -> Result<Vec<Goal>> {
        let active_goals = self.goals.iter()
            .filter(|g| g.status == GoalStatus::Active)
            .cloned()
            .collect::<Vec<_>>();
        
        // Sort by priority (highest first)
        let mut sorted_goals = active_goals;
        sorted_goals.sort_by(|a, b| b.priority.cmp(&a.priority));
        
        Ok(sorted_goals)
    }

    /// Update a goal's status
    pub async fn update_goal_status(&mut self, goal_id: &str, status: GoalStatus) -> Result<()> {
        if let Some(goal) = self.goals.iter_mut().find(|g| g.id == goal_id) {
            goal.status = status.clone();
            
            // If completed, set the completion timestamp
            if status == GoalStatus::Completed {
                goal.completed_at = Some(Utc::now());
            }
        }
        
        Ok(())
    }

    /// Update a goal's progress
    pub async fn update_goal_progress(&mut self, goal_id: &str, progress: f32) -> Result<()> {
        if let Some(goal) = self.goals.iter_mut().find(|g| g.id == goal_id) {
            goal.progress = progress;
            
            // If progress is 1.0, mark as completed
            if progress >= 1.0 {
                goal.status = GoalStatus::Completed;
                goal.completed_at = Some(Utc::now());
            }
        }
        
        Ok(())
    }

    /// Link a thought to a goal
    pub async fn link_thought_to_goal(&mut self, goal_id: &str, thought_id: &Uuid) -> Result<()> {
        if let Some(goal) = self.goals.iter_mut().find(|g| g.id == goal_id) {
            goal.related_thoughts.push(thought_id.to_string());
        }
        
        Ok(())
    }

    /// Get goals related to a specific thought
    pub async fn get_goals_for_thought(&self, thought_id: &Uuid) -> Result<Vec<Goal>> {
        let thought_id_str = thought_id.to_string();
        
        let related_goals = self.goals.iter()
            .filter(|g| g.related_thoughts.contains(&thought_id_str))
            .cloned()
            .collect::<Vec<_>>();
            
        Ok(related_goals)
    }
    
    // Chat message methods

    /// Store a new chat message
    pub async fn store_chat_message(&mut self, sender: ChatSender, content: &str) -> Result<Uuid> {
        let id = Uuid::new_v4();
        
        let message = ChatMessage {
            id,
            sender,
            content: content.to_string(),
            timestamp: Utc::now(),
            read: false,
            metadata: serde_json::json!({}),
        };
        
        self.chat_messages.push(message);
        
        Ok(id)
    }
    
    /// Mark a chat message as read
    pub async fn mark_message_read(&mut self, id: &Uuid) -> Result<()> {
        if let Some(message) = self.chat_messages.iter_mut().find(|m| m.id == *id) {
            message.read = true;
        }
        
        Ok(())
    }
    
    /// Get recent chat messages
    pub async fn get_recent_chat_messages(&self, limit: usize) -> Result<Vec<ChatMessage>> {
        let mut messages = self.chat_messages.clone();
        messages.sort_by(|a, b| b.timestamp.cmp(&a.timestamp)); // Most recent first
        
        Ok(messages.into_iter().take(limit).collect())
    }
    
    /// Get unread chat messages
    pub async fn get_unread_chat_messages(&self) -> Result<Vec<ChatMessage>> {
        let mut messages = self.chat_messages.iter()
            .filter(|m| !m.read)
            .cloned()
            .collect::<Vec<_>>();
            
        messages.sort_by(|a, b| b.timestamp.cmp(&a.timestamp)); // Most recent first
        
        Ok(messages)
    }
    /// Create a new memory system
    pub async fn new() -> Result<Self> {
        // In a real implementation, this would initialize a database connection
        Ok(Self {
            thoughts: Vec::new(),
            memories: Vec::new(),
            tool_executions: Vec::new(),
            chat_messages: Vec::new(),
            goals: Vec::new(),
        })
    }
    
    /// Store a thought in the memory system
    pub async fn store_thought(&mut self, thought: &Thought) -> Result<Uuid> {
        // Generate an ID for the thought
        let id = Uuid::new_v4();
        
        // Create a copy of the thought with the ID
        let mut new_thought = thought.clone();
        new_thought.id = Some(id);
        new_thought.created_at = Some(Utc::now());
        
        // Store the thought
        self.thoughts.push(new_thought);
        
        Ok(id)
    }
    
    /// Update a thought's stage in the memory system
    pub async fn update_thought_stage(&mut self, id: &Uuid, stage: &str) -> Result<()> {
        // Find the thought with the given ID
        if let Some(thought) = self.thoughts.iter_mut().find(|t| t.id == Some(*id)) {
            thought.pipeline_stage = stage.to_string();
            
            // Log the stage update for debugging
            fn log_memory(msg: &str) -> std::io::Result<()> {
                use std::fs::OpenOptions;
                use std::io::Write;
                let mut file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("memory_debug.log")?;
                    
                let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
                writeln!(file, "[{}] {}", timestamp, msg)?;
                
                Ok(())
            }
            
            let _ = log_memory(&format!(
                "UPDATED THOUGHT STAGE - ID: {}, New Stage: {}, Content: {}",
                id, stage, thought.content.chars().take(20).collect::<String>()
            ));
        } else {
            // Log that we couldn't find the thought
            fn log_memory(msg: &str) -> std::io::Result<()> {
                use std::fs::OpenOptions;
                use std::io::Write;
                let mut file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("memory_debug.log")?;
                    
                let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
                writeln!(file, "[{}] {}", timestamp, msg)?;
                
                Ok(())
            }
            
            let _ = log_memory(&format!(
                "ERROR - THOUGHT NOT FOUND FOR STAGE UPDATE - ID: {}, Stage: {}",
                id, stage
            ));
        }
        
        Ok(())
    }
    
    /// Mark a thought as externalized
    pub async fn mark_thought_externalized(&mut self, id: &Uuid) -> Result<()> {
        // Find the thought with the given ID
        if let Some(thought) = self.thoughts.iter_mut().find(|t| t.id == Some(*id)) {
            let metadata = serde_json::from_value::<std::collections::HashMap<String, Value>>(thought.metadata.clone())
                .unwrap_or_default();
            
            // Create a new metadata value with the externalized flag
            let mut new_metadata = metadata;
            new_metadata.insert("externalized".to_string(), serde_json::json!(true));
            new_metadata.insert("externalized_at".to_string(), serde_json::json!(Utc::now().to_rfc3339()));
            
            thought.metadata = serde_json::to_value(new_metadata).unwrap_or_default();
        }
        
        Ok(())
    }
    
    /// Get a thought by ID
    pub async fn get_thought(&self, id: &Uuid) -> Option<Thought> {
        self.thoughts.iter().find(|t| t.id == Some(*id)).cloned()
    }
    
    /// Get recent thoughts balanced across all pipeline stages
    pub async fn get_recent_thoughts(&self, limit: usize) -> Result<Vec<Thought>> {
        // If limit is 0, return empty array (used for counting)
        if limit == 0 {
            return Ok(Vec::new());
        }
        
        // Log counts - create a simple tui_debug_log equivalent
        fn log_memory(msg: &str) -> std::io::Result<()> {
            use std::fs::OpenOptions;
            use std::io::Write;
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open("memory_debug.log")?;
                
            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
            writeln!(file, "[{}] {}", timestamp, msg)?;
            
            Ok(())
        }
        
        // Log initial state
        let base_count = self.thoughts.iter().filter(|t| t.pipeline_stage == "base_instinct").count();
        let values_count = self.thoughts.iter().filter(|t| t.pipeline_stage == "values_assessment").count();
        let psych_count = self.thoughts.iter().filter(|t| t.pipeline_stage == "psychological_processing").count();
        let external_count = self.thoughts.iter().filter(|t| t.pipeline_stage == "externalization_decision").count();
        
        let _ = log_memory(&format!(
            "MEMORY GET_RECENT_THOUGHTS - Total: {}, Base: {}, Values: {}, Psych: {}, External: {}, Limit: {}",
            self.thoughts.len(), base_count, values_count, psych_count, external_count, limit
        ));
        
        // Clone all thoughts for processing
        let all_thoughts = self.thoughts.clone();
        
        // Always use balanced approach regardless of limit size
        let stages = ["base_instinct", "values_assessment", "psychological_processing", "externalization_decision"];
        let mut result = Vec::new();
        
        // Log the stage counts for debugging
        let base_count = all_thoughts.iter().filter(|t| t.pipeline_stage == "base_instinct").count();
        let values_count = all_thoughts.iter().filter(|t| t.pipeline_stage == "values_assessment").count();
        let psych_count = all_thoughts.iter().filter(|t| t.pipeline_stage == "psychological_processing").count();
        let external_count = all_thoughts.iter().filter(|t| t.pipeline_stage == "externalization_decision").count();
        
        let _ = log_memory(&format!(
            "RAW COUNTS - Base: {}, Values: {}, Psych: {}, External: {}",
            base_count, values_count, psych_count, external_count
        ));
        
        // Calculate minimum per stage to ensure representation
        let min_per_stage = std::cmp::max(5, limit / stages.len());
        
        // First pass: get some thoughts from each stage, sorted by recency
        for stage in &stages {
            let mut stage_thoughts = all_thoughts.iter()
                .filter(|t| t.pipeline_stage == *stage)
                .cloned()
                .collect::<Vec<_>>();
                
            // Log the count for this stage
            let _ = log_memory(&format!(
                "STAGE {} - Found {} thoughts", 
                stage, stage_thoughts.len()
            ));
                
            // Sort by creation time (newest first)
            stage_thoughts.sort_by(|a, b| {
                let a_time = a.created_at.unwrap_or_else(|| Utc::now());
                let b_time = b.created_at.unwrap_or_else(|| Utc::now());
                b_time.cmp(&a_time)
            });
            
            // Take up to min_per_stage thoughts
            let thoughts_to_add = stage_thoughts.into_iter().take(min_per_stage).collect::<Vec<_>>();
            
            // Log how many we're adding from this stage
            let _ = log_memory(&format!(
                "ADDING {} thoughts from stage {}", 
                thoughts_to_add.len(), stage
            ));
            
            // Extend results with thoughts from this stage
            result.extend(thoughts_to_add);
        }
        
        // Fill remaining slots with newest thoughts from any stage
        if result.len() < limit {
            // Create a set of IDs we already have
            let existing_ids: std::collections::HashSet<_> = result.iter()
                .filter_map(|t| t.id)
                .collect();
                
            // Get remaining thoughts sorted by recency
            let mut remaining_thoughts = all_thoughts.iter()
                .filter(|t| t.id.map_or(true, |id| !existing_ids.contains(&id)))
                .cloned()
                .collect::<Vec<_>>();
                
            // Sort by creation time (newest first)
            remaining_thoughts.sort_by(|a, b| {
                let a_time = a.created_at.unwrap_or_else(|| Utc::now());
                let b_time = b.created_at.unwrap_or_else(|| Utc::now());
                b_time.cmp(&a_time)
            });
            
            // Add remaining thoughts up to the limit
            let remaining_to_add = limit.saturating_sub(result.len());
            result.extend(remaining_thoughts.into_iter().take(remaining_to_add));
        }
        
        // Ensure we don't exceed the limit
        while result.len() > limit {
            result.pop();
        }
        
        // Sort final result by pipeline stage (to group thoughts correctly)
        result.sort_by(|a, b| {
            // Primary sort by pipeline stage index
            let a_stage_idx = stages.iter().position(|s| *s == a.pipeline_stage).unwrap_or(999);
            let b_stage_idx = stages.iter().position(|s| *s == b.pipeline_stage).unwrap_or(999);
            
            let stage_cmp = a_stage_idx.cmp(&b_stage_idx);
            if stage_cmp != std::cmp::Ordering::Equal {
                return stage_cmp;
            }
            
            // Secondary sort by creation time (newest first)
            let a_time = a.created_at.unwrap_or_else(|| Utc::now());
            let b_time = b.created_at.unwrap_or_else(|| Utc::now());
            b_time.cmp(&a_time)
        });
        
        // Log what we're returning
        let result_base = result.iter().filter(|t| t.pipeline_stage == "base_instinct").count();
        let result_values = result.iter().filter(|t| t.pipeline_stage == "values_assessment").count();
        let result_psych = result.iter().filter(|t| t.pipeline_stage == "psychological_processing").count();
        let result_external = result.iter().filter(|t| t.pipeline_stage == "externalization_decision").count();
        
        let _ = log_memory(&format!(
            "MEMORY RETURNING - Total: {}, Base: {}, Values: {}, Psych: {}, External: {}",
            result.len(), result_base, result_values, result_psych, result_external
        ));
        
        if !result.is_empty() {
            for (i, thought) in result.iter().enumerate().take(3) {
                let _ = log_memory(&format!(
                    "SAMPLE THOUGHT {}: stage={}, content={}",
                    i, thought.pipeline_stage, 
                    thought.content.chars().take(30).collect::<String>()
                ));
            }
        }
        
        Ok(result)
    }
    
    /// Store a memory
    pub async fn store_memory(&mut self, memory: &Memory) -> Result<Uuid> {
        // Generate an ID for the memory
        let id = Uuid::new_v4();
        
        // Create a copy of the memory with the ID
        let mut new_memory = memory.clone();
        new_memory.id = Some(id);
        new_memory.created_at = Some(Utc::now());
        new_memory.last_accessed = Some(Utc::now());
        
        // Store the memory
        self.memories.push(new_memory);
        
        Ok(id)
    }
    
    /// Get important memories
    pub async fn get_important_memories(&self, limit: usize) -> Result<Vec<Memory>> {
        // Sort memories by importance and take the most important ones
        let mut memories = self.memories.clone();
        memories.sort_by(|a, b| b.importance.cmp(&a.importance));
        
        Ok(memories.into_iter().take(limit).collect())
    }
    
    /// Log a tool execution
    pub async fn log_tool_execution(
        &mut self,
        tool_name: &str,
        args: &Value,
        result: &str,
        success: bool,
        duration: std::time::Duration,
        thought_id: Option<&Uuid>,
    ) -> Result<()> {
        // Create a new tool execution record
        let execution = ToolExecution {
            id: Uuid::new_v4(),
            tool_name: tool_name.to_string(),
            args: args.clone(),
            result: result.to_string(),
            success,
            duration_ms: duration.as_millis() as u64,
            executed_at: Utc::now(),
            thought_id: thought_id.cloned(),
        };
        
        // Store the execution
        self.tool_executions.push(execution);
        
        Ok(())
    }
    
    /// Get recent tool executions
    pub async fn get_recent_tool_executions(&self, limit: usize) -> Result<Vec<ToolExecution>> {
        // Sort executions by execution time and take the most recent ones
        let mut executions = self.tool_executions.clone();
        executions.sort_by(|a, b| b.executed_at.cmp(&a.executed_at));
        
        Ok(executions.into_iter().take(limit).collect())
    }
}

/// Represents a thought stored in the memory system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thought {
    /// Unique identifier for the thought
    pub id: Option<Uuid>,
    
    /// The actual content of the thought
    pub content: String,
    
    /// Type of thought (curiosity, planning, reaction, etc.)
    pub thought_type: String,
    
    /// Current stage in the thought pipeline
    pub pipeline_stage: String,
    
    /// Additional metadata about the thought
    pub metadata: Value,
    
    /// When the thought was created
    pub created_at: Option<DateTime<Utc>>,
}

/// Represents a memory stored in the memory system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    /// Unique identifier for the memory
    pub id: Option<Uuid>,
    
    /// The actual content of the memory
    pub content: String,
    
    /// Importance level (1-10)
    pub importance: i32,
    
    /// When the memory was created
    pub created_at: Option<DateTime<Utc>>,
    
    /// When the memory was last accessed
    pub last_accessed: Option<DateTime<Utc>>,
    
    /// Number of times the memory has been accessed
    pub access_count: i32,
    
    /// Tags for categorization
    pub tags: Value,
    
    /// Associations with other memories
    pub associations: Value,
}

impl Default for Memory {
    fn default() -> Self {
        Self {
            id: None,
            content: String::new(),
            importance: 5, // Medium importance by default
            created_at: None,
            last_accessed: None,
            access_count: 0,
            tags: serde_json::json!({}),
            associations: serde_json::json!([]),
        }
    }
}

/// Represents a tool execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecution {
    /// Unique identifier for the execution
    pub id: Uuid,
    
    /// Name of the tool that was executed
    pub tool_name: String,
    
    /// Arguments passed to the tool
    pub args: Value,
    
    /// Result of the tool execution
    pub result: String,
    
    /// Whether the execution was successful
    pub success: bool,
    
    /// Duration of the execution in milliseconds
    pub duration_ms: u64,
    
    /// When the tool was executed
    pub executed_at: DateTime<Utc>,
    
    /// ID of the thought that triggered the tool execution
    pub thought_id: Option<Uuid>,
}

/// Represents a message in the user chat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Unique identifier for the message
    pub id: Uuid,
    
    /// Who sent the message (brain or user)
    pub sender: ChatSender,
    
    /// Content of the message
    pub content: String,
    
    /// When the message was sent
    pub timestamp: DateTime<Utc>,
    
    /// Whether the message has been read
    pub read: bool,
    
    /// Optional metadata (like links to thoughts that generated this)
    pub metadata: Value,
}

/// Represents who sent a chat message
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChatSender {
    /// Message from the brain
    Brain,
    
    /// Message from the user
    User,
}

/// Goal structure shared with brain_core crate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    /// Unique identifier for the goal
    pub id: String,
    
    /// The description of the goal
    pub description: String,
    
    /// Priority level (1-10)
    pub priority: u8,
    
    /// Current status of the goal
    pub status: GoalStatus,
    
    /// When the goal was created
    pub created_at: DateTime<Utc>,
    
    /// When the goal was completed (if it has been)
    pub completed_at: Option<DateTime<Utc>>,
    
    /// IDs of thoughts related to this goal
    pub related_thoughts: Vec<String>,
    
    /// Progress toward completion (0.0 - 1.0)
    pub progress: f32,
    
    /// Optional parent goal ID for hierarchical goals
    pub parent_goal: Option<String>,
    
    /// Child goal IDs
    pub sub_goals: Vec<String>,
}

/// Goal status enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GoalStatus {
    Active,
    Completed,
    Abandoned,
    Blocked,
}