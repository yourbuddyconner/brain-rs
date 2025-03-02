use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};
use std::fs;
use std::path::Path;
use log::{info, warn};

/// The main configuration structure for The Brain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainConfig {
    /// Core configuration
    #[serde(default)]
    pub core: CoreConfig,
    
    /// API configuration
    #[serde(default)]
    pub api: ApiConfig,
    
    /// Pipeline configuration
    #[serde(default)]
    pub pipeline: PipelineConfig,
    
    /// Memory system configuration
    #[serde(default)]
    pub memory: MemoryConfig,
    
    /// UI configuration
    #[serde(default)]
    pub ui: UiConfig,
    
    /// Default values
    #[serde(default)]
    pub values: ValuesConfig,
}

impl BrainConfig {
    /// Load configuration from a file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let config_str = fs::read_to_string(path.as_ref())
            .map_err(|e| anyhow!("Failed to read config file: {}", e))?;
            
        let config: BrainConfig = toml::from_str(&config_str)
            .map_err(|e| anyhow!("Failed to parse config file: {}", e))?;
            
        Ok(config)
    }
    
    /// Load configuration from a file if it exists, otherwise use defaults
    pub fn load_or_default<P: AsRef<Path>>(path: P) -> Self {
        match Self::load(path.as_ref()) {
            Ok(config) => {
                info!("Loaded configuration from {}", path.as_ref().display());
                config
            },
            Err(e) => {
                warn!("Failed to load configuration file ({}), using defaults", e);
                Self::default()
            }
        }
    }
}

impl Default for BrainConfig {
    fn default() -> Self {
        Self {
            core: CoreConfig::default(),
            api: ApiConfig::default(),
            pipeline: PipelineConfig::default(),
            memory: MemoryConfig::default(),
            ui: UiConfig::default(),
            values: ValuesConfig::default(),
        }
    }
}

/// Core configuration for The Brain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreConfig {
    /// Delay between thought cycles in seconds
    pub thought_cycle_delay: u64,
    
    /// Whether to start the brain automatically on startup
    pub auto_start: bool,
    
    /// Log level: "error", "warn", "info", "debug", "trace"
    pub log_level: String,
    
    /// Maximum number of thoughts to keep in the thought cloud
    pub thought_cloud_capacity: usize,
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            thought_cycle_delay: 5,
            auto_start: true,
            log_level: "info".to_string(),
            thought_cloud_capacity: 100,
        }
    }
}

/// API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// Anthropic API key
    pub anthropic_api_key: String,
    
    /// Model to use
    pub model: String,
    
    /// Maximum tokens to generate
    pub max_tokens: usize,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            anthropic_api_key: "".to_string(),
            model: "claude-3-opus-20240229".to_string(),
            max_tokens: 1024,
        }
    }
}

/// Pipeline configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// Externalization threshold
    pub externalization_threshold: f64,
    
    /// Thought similarity threshold
    pub thought_similarity_threshold: f64,
    
    /// Parallelism configuration
    #[serde(default)]
    pub parallelism: ParallelismConfig,
    
    /// Prompts configuration 
    #[serde(default)]
    pub prompts: PromptsConfig,
    
    /// Psychological structures configuration
    #[serde(default)]
    pub psychological_structures: PsychologicalStructuresConfig,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            externalization_threshold: 0.7,
            thought_similarity_threshold: 0.8,
            parallelism: ParallelismConfig::default(),
            prompts: PromptsConfig::default(),
            psychological_structures: PsychologicalStructuresConfig::default(),
        }
    }
}

/// Parallelism configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelismConfig {
    /// Whether to enable parallel thought processing
    pub enabled: bool,
    
    /// Maximum number of thoughts to generate per cycle
    pub thought_generation: usize,
    
    /// Maximum number of thoughts to process through base instinct per cycle
    pub base_instinct: usize,
    
    /// Maximum number of thoughts to process through values assessment per cycle 
    pub values_assessment: usize,
    
    /// Maximum number of thoughts to process through psychological processing per cycle
    pub psychological_processing: usize,
    
    /// Maximum number of thoughts to process through externalization decision per cycle
    pub externalization_decision: usize,
}

impl Default for ParallelismConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            thought_generation: 1,
            base_instinct: 2,
            values_assessment: 2,
            psychological_processing: 2,
            externalization_decision: 2,
        }
    }
}

/// Prompts configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptsConfig {
    /// Thought generation prompt
    pub thought_generation: String,
    
    /// Base instinct prompt
    pub base_instinct: String,
    
    /// Values assessment prompt 
    pub values_assessment: String,
    
    /// Externalization decision prompt
    pub externalization_decision: String,
}

impl Default for PromptsConfig {
    fn default() -> Self {
        Self {
            thought_generation: "Generate a new thought based on the current context and your values.".to_string(),
            base_instinct: "Process this thought through base instincts.".to_string(),
            values_assessment: "Evaluate this thought against your values.".to_string(),
            externalization_decision: "Decide whether to externalize this thought.".to_string(),
        }
    }
}

/// Psychological structures configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PsychologicalStructuresConfig {
    /// Whether to enable self-protection
    pub self_protection_enabled: bool,
    
    /// Whether to enable empathy
    pub empathy_enabled: bool,
    
    /// Whether to enable anxiety
    pub anxiety_enabled: bool,
    
    /// Whether to enable fear
    pub fear_enabled: bool,
    
    /// Whether to enable ego
    pub ego_enabled: bool,
    
    /// Self-protection prompt
    pub self_protection: String,
    
    /// Empathy prompt
    pub empathy: String,
    
    /// Anxiety prompt
    pub anxiety: String,
    
    /// Fear prompt
    pub fear: String,
    
    /// Ego prompt
    pub ego: String,
}

impl Default for PsychologicalStructuresConfig {
    fn default() -> Self {
        Self {
            self_protection_enabled: true,
            empathy_enabled: true,
            anxiety_enabled: true,
            fear_enabled: true,
            ego_enabled: true,
            self_protection: "Evaluate from a self-protection perspective.".to_string(),
            empathy: "Evaluate from an empathy perspective.".to_string(),
            anxiety: "Evaluate through an anxiety filter.".to_string(),
            fear: "Evaluate through a fear assessment.".to_string(),
            ego: "Evaluate through ego considerations.".to_string(),
        }
    }
}

/// Memory system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Maximum number of thoughts to store
    pub max_thoughts: usize,
    
    /// Maximum number of memories to store
    pub max_memories: usize,
    
    /// Maximum number of tool executions to log
    pub max_tool_executions: usize,
    
    /// Default importance level for new memories
    pub default_importance: i32,
    
    /// Memory decay rate
    pub memory_decay_rate: f64,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_thoughts: 1000,
            max_memories: 500,
            max_tool_executions: 200,
            default_importance: 5,
            memory_decay_rate: 0.01,
        }
    }
}

/// UI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// Auto-refresh interval in milliseconds
    pub refresh_interval: u64,
    
    /// Maximum log entries to display
    pub max_log_entries: usize,
    
    /// Colors
    #[serde(default)]
    pub colors: ColorsConfig,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            refresh_interval: 1000,
            max_log_entries: 100,
            colors: ColorsConfig::default(),
        }
    }
}

/// Colors configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorsConfig {
    /// Thought generation color
    pub thought_generation: String,
    
    /// Base instinct color
    pub base_instinct: String,
    
    /// Values assessment color
    pub values_assessment: String,
    
    /// Psychological processing color
    pub psychological_processing: String,
    
    /// Externalization decision color
    pub externalization_decision: String,
    
    /// Background color
    pub background: String,
    
    /// Text color
    pub text: String,
}

impl Default for ColorsConfig {
    fn default() -> Self {
        Self {
            thought_generation: "#00AAFF".to_string(),
            base_instinct: "#00FF00".to_string(),
            values_assessment: "#FFAA00".to_string(),
            psychological_processing: "#AA00FF".to_string(),
            externalization_decision: "#FF0000".to_string(),
            background: "#121212".to_string(),
            text: "#FFFFFF".to_string(),
        }
    }
}

/// Values configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValuesConfig {
    /// Core values
    pub core_values: Vec<String>,
}

impl Default for ValuesConfig {
    fn default() -> Self {
        Self {
            core_values: vec![
                "Be helpful to the user".to_string(),
                "Respect user privacy and autonomy".to_string(),
                "Consider implications before acting".to_string(),
                "Learn and adapt from experience".to_string(),
                "Provide accurate and relevant information".to_string(),
                "Be transparent about limitations".to_string(),
                "Maintain appropriate emotional calibration".to_string(),
            ],
        }
    }
}