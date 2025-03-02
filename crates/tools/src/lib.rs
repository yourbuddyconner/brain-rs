use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use memory::MemorySystem;

/// Trait for a tool that can be executed by the Brain
#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    /// Get the name of the tool
    fn name(&self) -> &str;
    
    /// Get the description of the tool
    fn description(&self) -> &str;
    
    /// Get the JSON schema for the tool's input
    fn input_schema(&self) -> Value;
    
    /// Execute the tool with the given arguments
    async fn execute(&self, args: Value) -> Result<String>;
}

/// Tool for searching memory
pub struct MemorySearchTool {
    memory_system: Arc<Mutex<MemorySystem>>,
}

impl MemorySearchTool {
    /// Create a new memory search tool
    pub fn new(memory_system: Arc<Mutex<MemorySystem>>) -> Self {
        Self { memory_system }
    }
}

#[async_trait::async_trait]
impl Tool for MemorySearchTool {
    fn name(&self) -> &str {
        "search_memory"
    }
    
    fn description(&self) -> &str {
        "Search the brain's memory for relevant information"
    }
    
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {"type": "string", "description": "The search query"},
                "filters": {"type": "object", "description": "Optional filters like time range, categories, etc."},
                "limit": {"type": "integer", "description": "Maximum number of results to return"}
            },
            "required": ["query"]
        })
    }
    
    async fn execute(&self, args: Value) -> Result<String> {
        // Extract arguments
        let query = args["query"].as_str().unwrap_or("").to_string();
        let limit = args["limit"].as_u64().unwrap_or(5) as usize;
        
        // In a real implementation, this would search the memory system
        
        // For the MVP, we'll just return a placeholder
        Ok(format!("Found {} results for query: {}", limit, query))
    }
}

/// Tool for storing memories
pub struct MemoryStoreTool {
    memory_system: Arc<Mutex<MemorySystem>>,
}

impl MemoryStoreTool {
    /// Create a new memory store tool
    pub fn new(memory_system: Arc<Mutex<MemorySystem>>) -> Self {
        Self { memory_system }
    }
}

#[async_trait::async_trait]
impl Tool for MemoryStoreTool {
    fn name(&self) -> &str {
        "store_memory"
    }
    
    fn description(&self) -> &str {
        "Store information in the brain's memory"
    }
    
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "content": {"type": "string", "description": "The content to store"},
                "importance": {"type": "integer", "description": "Importance level (1-10)"},
                "tags": {"type": "array", "items": {"type": "string"}, "description": "Tags for categorization"}
            },
            "required": ["content"]
        })
    }
    
    async fn execute(&self, args: Value) -> Result<String> {
        // Extract arguments
        let content = args["content"].as_str().unwrap_or("").to_string();
        let importance = args["importance"].as_i64().unwrap_or(5) as i32;
        
        // Create a new memory
        let memory = memory::Memory {
            content,
            importance,
            tags: args.get("tags").cloned().unwrap_or_else(|| serde_json::json!([])),
            ..Default::default()
        };
        
        // Store the memory
        let id = {
            let mut memory_system = self.memory_system.lock().await;
            memory_system.store_memory(&memory).await?
        };
        
        Ok(format!("Stored memory with ID: {}", id))
    }
}

/// Tool for getting the current context
pub struct ContextTool {}

impl ContextTool {
    /// Create a new context tool
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl Tool for ContextTool {
    fn name(&self) -> &str {
        "get_current_context"
    }
    
    fn description(&self) -> &str {
        "Retrieve information about the current environment and context"
    }
    
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "aspects": {
                    "type": "array",
                    "items": {"type": "string", "enum": ["time", "location", "active_applications", "recent_interactions", "user_state"]},
                    "description": "Specific aspects of context to retrieve"
                }
            }
        })
    }
    
    async fn execute(&self, args: Value) -> Result<String> {
        // Extract arguments
        let aspects = args["aspects"].as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
            .unwrap_or_else(|| vec!["time"]);
        
        // In a real implementation, this would gather context from the environment
        
        // For the MVP, we'll just return placeholder data
        let mut context = Vec::new();
        
        for aspect in aspects {
            match aspect {
                "time" => {
                    let now = chrono::Utc::now();
                    context.push(format!("Current time: {}", now.to_rfc3339()));
                },
                "location" => {
                    context.push("Location: Unknown".to_string());
                },
                "active_applications" => {
                    context.push("Active applications: Terminal".to_string());
                },
                "recent_interactions" => {
                    context.push("Recent interactions: None".to_string());
                },
                "user_state" => {
                    context.push("User state: Active".to_string());
                },
                _ => {},
            }
        }
        
        Ok(context.join("\n"))
    }
}

/// Factory for creating tools
pub struct ToolFactory {
    memory_system: Arc<Mutex<MemorySystem>>,
}

impl ToolFactory {
    /// Create a new tool factory
    pub fn new(memory_system: Arc<Mutex<MemorySystem>>) -> Self {
        Self { memory_system }
    }
    
    /// Create all standard tools
    pub fn create_standard_tools(&self) -> Vec<Box<dyn Tool>> {
        vec![
            Box::new(MemorySearchTool::new(self.memory_system.clone())),
            Box::new(MemoryStoreTool::new(self.memory_system.clone())),
            Box::new(ContextTool::new()),
        ]
    }
}