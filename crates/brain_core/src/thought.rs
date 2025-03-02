use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::VecDeque;

/// Represents a single thought in the brain
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

impl Default for Thought {
    fn default() -> Self {
        Self {
            id: None,
            content: String::new(),
            thought_type: "general".to_string(),
            pipeline_stage: "base_instinct".to_string(), // This is correct - new thoughts should start in base_instinct
            metadata: serde_json::json!({}),
            created_at: Some(Utc::now()),
        }
    }
}

impl Thought {
    /// Create a new thought with the given content
    pub fn new(content: &str) -> Self {
        Self {
            content: content.to_string(),
            ..Default::default()
        }
    }
    
    /// Create a new thought with specified type
    pub fn with_type(content: &str, thought_type: &str) -> Self {
        Self {
            content: content.to_string(),
            thought_type: thought_type.to_string(),
            ..Default::default()
        }
    }
}

/// Represents a collection of thoughts that are waiting to be processed
pub struct ThoughtCloud {
    /// Queue of thoughts to be processed
    thoughts: VecDeque<Thought>,
    
    /// Maximum number of thoughts to keep in the cloud
    capacity: usize,
}

impl ThoughtCloud {
    /// Create a new thought cloud with default capacity
    pub fn new() -> Self {
        Self {
            thoughts: VecDeque::new(),
            capacity: 100, // Default capacity
        }
    }
    
    /// Create a new thought cloud with specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            thoughts: VecDeque::with_capacity(capacity),
            capacity,
        }
    }
    
    /// Add a thought to the cloud
    pub fn add_thought(&mut self, thought: Thought) {
        // If at capacity, remove the oldest thought
        if self.thoughts.len() >= self.capacity {
            self.thoughts.pop_back();
        }
        
        // Add the new thought to the front
        self.thoughts.push_front(thought);
    }
    
    /// Get the next thought to process
    pub fn next_thought(&mut self) -> Option<Thought> {
        self.thoughts.pop_back()
    }
    
    /// Check if the thought cloud is empty
    pub fn is_empty(&self) -> bool {
        self.thoughts.is_empty()
    }
    
    /// Get the number of thoughts in the cloud
    pub fn len(&self) -> usize {
        self.thoughts.len()
    }
    
    /// Get a list of all thoughts in the cloud
    pub fn list_thoughts(&self) -> Vec<Thought> {
        self.thoughts.iter().cloned().collect()
    }
    
    /// Get a reference to all thoughts in the cloud
    pub fn thoughts(&self) -> &VecDeque<Thought> {
        &self.thoughts
    }
    
    /// Remove a thought from the cloud by ID
    pub fn remove_thought(&mut self, id: &Option<Uuid>) {
        if let Some(thought_id) = id {
            self.thoughts.retain(|t| {
                if let Some(t_id) = t.id {
                    t_id != *thought_id
                } else {
                    true
                }
            });
        }
    }
}

/// Represents a processed thought with additional information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedThought {
    /// The thought that was processed
    pub thought: Thought,
    
    /// The original thought that was processed (for reference)
    pub original_thought: Thought,
    
    /// The result of processing the thought
    pub content: String,
    
    /// Results from tool invocations
    pub tool_results: Vec<Value>,
    
    /// Score indicating how suitable the thought is for externalization
    pub externalization_score: f64,
    
    /// Suggested action to take based on the thought
    pub suggested_action: Option<String>,
}

impl ProcessedThought {
    /// Create a new processed thought from an original thought
    pub fn new(thought: Thought, content: &str) -> Self {
        Self {
            original_thought: thought.clone(),
            thought,
            content: content.to_string(),
            tool_results: Vec::new(),
            externalization_score: 0.0,
            suggested_action: None,
        }
    }
    
    /// Set the externalization score
    pub fn with_externalization_score(mut self, score: f64) -> Self {
        self.externalization_score = score;
        self
    }
    
    /// Add a suggested action
    pub fn with_suggested_action(mut self, action: &str) -> Self {
        self.suggested_action = Some(action.to_string());
        self
    }
    
    /// Add tool results
    pub fn with_tool_results(mut self, results: Vec<Value>) -> Self {
        self.tool_results = results;
        self
    }
    
    /// Add a specific tool result
    pub fn add_tool_result(&mut self, result: Value) {
        self.tool_results.push(result);
    }
    
    /// Add metadata to the processed thought
    pub fn add_metadata(&mut self, key: &str, value: Value) {
        let mut metadata = serde_json::from_value::<std::collections::HashMap<String, Value>>(
            self.thought.metadata.clone()
        ).unwrap_or_default();
        
        metadata.insert(key.to_string(), value);
        self.thought.metadata = serde_json::to_value(metadata).unwrap_or_default();
    }
}