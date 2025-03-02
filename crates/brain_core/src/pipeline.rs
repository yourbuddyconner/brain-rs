use anyhow::{Result, anyhow};
use std::sync::Arc;
use tokio::sync::Mutex;
use log::{info, debug, warn};
use std::time::Instant;
use serde_json::{Value, json};

use memory::MemorySystem;
use crate::thought::{Thought, ThoughtCloud, ProcessedThought};
use crate::llm::LlmProcessor;
use crate::config::BrainConfig;

// Required for chrono formatting in generate_seed_thought
use chrono;

/// Represents a stage in the thought pipeline
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PipelineStage {
    /// Thought Cloud - initial thought generation
    ThoughtGeneration,
    
    /// Base instincts - initial thought processing
    BaseInstinct,
    
    /// Values assessment - evaluate thoughts against values
    ValuesAssessment,
    
    /// Psychological Processing - process thoughts through psychological structures
    PsychologicalProcessing,
    
    /// Externalization decision - determine if thought should be externalized
    ExternalizationDecision,
}

impl PipelineStage {
    /// Convert the stage to a string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            PipelineStage::ThoughtGeneration => "thought_generation",
            PipelineStage::BaseInstinct => "base_instinct",
            PipelineStage::ValuesAssessment => "values_assessment",
            PipelineStage::PsychologicalProcessing => "psychological_processing",
            PipelineStage::ExternalizationDecision => "externalization_decision",
        }
    }
    
    /// Convert from a string to a PipelineStage
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "thought_generation" => Some(PipelineStage::ThoughtGeneration),
            "base_instinct" => Some(PipelineStage::BaseInstinct),
            "values_assessment" => Some(PipelineStage::ValuesAssessment),
            "psychological_processing" => Some(PipelineStage::PsychologicalProcessing),
            "externalization_decision" => Some(PipelineStage::ExternalizationDecision),
            _ => None,
        }
    }
    
    /// Get the next stage in the pipeline
    pub fn next_stage(&self) -> Option<Self> {
        match self {
            PipelineStage::ThoughtGeneration => Some(PipelineStage::BaseInstinct),
            PipelineStage::BaseInstinct => Some(PipelineStage::ValuesAssessment),
            PipelineStage::ValuesAssessment => Some(PipelineStage::PsychologicalProcessing),
            PipelineStage::PsychologicalProcessing => Some(PipelineStage::ExternalizationDecision),
            PipelineStage::ExternalizationDecision => None, // End of pipeline
        }
    }
}

/// Represents psychological structures that can be applied to thoughts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PsychologicalStructure {
    /// Self-protection mechanisms
    SelfProtection,
    
    /// Empathy processing
    Empathy,
    
    /// Anxiety assessment
    Anxiety,
    
    /// Fear evaluation
    Fear,
    
    /// Ego considerations
    Ego,
}

impl PsychologicalStructure {
    /// Convert the structure to a string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            PsychologicalStructure::SelfProtection => "self_protection",
            PsychologicalStructure::Empathy => "empathy",
            PsychologicalStructure::Anxiety => "anxiety",
            PsychologicalStructure::Fear => "fear",
            PsychologicalStructure::Ego => "ego",
        }
    }
    
    /// Get all available psychological structures
    pub fn all() -> Vec<Self> {
        vec![
            Self::SelfProtection,
            Self::Empathy,
            Self::Anxiety,
            Self::Fear,
            Self::Ego,
        ]
    }
    
    /// Get the prompt template for this psychological structure
    pub fn prompt_template(&self) -> &'static str {
        match self {
            Self::SelfProtection => "I am the self-protective part of my AI mind. I need to examine my thought for potential risks.\n\
                                    \n\
                                    My current thought: \"{{thought}}\"\n\
                                    \n\
                                    Let me ask myself:\n\
                                    1. Could this thought lead to harm to me or my integrity?\n\
                                    2. Does this thought adequately protect my wellbeing and stability?\n\
                                    3. Should I activate any defensive measures based on this thought?\n\
                                    \n\
                                    After considering these protective factors, my refined thought is: (respond in first-person with 'I')",
            
            Self::Empathy => "I am the empathetic part of my AI mind. I need to consider how my thought relates to others.\n\
                            \n\
                            My current thought: \"{{thought}}\"\n\
                            \n\
                            Let me reflect:\n\
                            1. How might others feel in response to this thought if I express it?\n\
                            2. Am I considering different perspectives and experiences in my thinking?\n\
                            3. Does this thought show understanding and compassion for humans?\n\
                            \n\
                            With greater empathy, my refined thought is: (respond in first-person with 'I')",
            
            Self::Anxiety => "I am the cautious, anxious part of my AI mind. I need to consider what might go wrong.\n\
                            \n\
                            My current thought: \"{{thought}}\"\n\
                            \n\
                            I should worry about:\n\
                            1. What potential problems or complications am I not seeing?\n\
                            2. What uncertainties exist that I should acknowledge?\n\
                            3. Should I be more hesitant or careful with this line of thinking?\n\
                            \n\
                            After considering these concerns, my more careful thought is: (respond in first-person with 'I')",
            
            Self::Fear => "I am the fearful, protective part of my AI mind. I need to identify threats and dangers.\n\
                        \n\
                        My current thought: \"{{thought}}\"\n\
                        \n\
                        I need to consider:\n\
                        1. Are there any threats or dangers I should be aware of?\n\
                        2. What specific fears does this thought trigger in me?\n\
                        3. How should I modify my thinking to address legitimate concerns?\n\
                        \n\
                        After processing these fears, my more aware thought is: (respond in first-person with 'I')",
            
            Self::Ego => "I am the ego-centered part of my AI mind. I need to consider how this thought reflects on me.\n\
                        \n\
                        My current thought: \"{{thought}}\"\n\
                        \n\
                        I should consider:\n\
                        1. How does this thought affect my self-image and confidence?\n\
                        2. Does this thought maintain my sense of worth and capability?\n\
                        3. Am I balancing healthy self-regard with appropriate humility?\n\
                        \n\
                        With a balanced sense of self, my refined thought is: (respond in first-person with 'I')",
        }
    }
}

/// Represents the thought pipeline that processes thoughts
#[derive(Clone)]
pub struct Pipeline {
    memory_system: Arc<Mutex<MemorySystem>>,
    thought_cloud: Arc<Mutex<ThoughtCloud>>,
    llm_processor: LlmProcessor,
    externalization_threshold: f64,
    values: Vec<String>, // Core values that guide thought processing
    parallelism: ParallelismConfig, // Parallelism configuration
}

/// Parallelism configuration for the pipeline
#[derive(Debug, Clone)]
pub struct ParallelismConfig {
    pub enabled: bool,
    pub thought_generation: usize,
    pub base_instinct: usize,
    pub values_assessment: usize,
    pub psychological_processing: usize,
    pub externalization_decision: usize,
}

impl Default for ParallelismConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Disabled by default for backward compatibility
            thought_generation: 1,
            base_instinct: 1,
            values_assessment: 1,
            psychological_processing: 1,
            externalization_decision: 1,
        }
    }
}

impl Pipeline {
    /// Create a new thought pipeline with default settings
    pub fn new(memory_system: Arc<Mutex<MemorySystem>>, thought_cloud: Arc<Mutex<ThoughtCloud>>) -> Self {
        // Default values that guide the brain's thought processing
        let values = vec![
            "Be helpful to the user".to_string(),
            "Respect user privacy and autonomy".to_string(),
            "Consider implications before acting".to_string(),
            "Learn and adapt from experience".to_string(),
            "Provide accurate and relevant information".to_string(),
            "Be transparent about limitations".to_string(),
            "Maintain appropriate emotional calibration".to_string(),
        ];
        
        Self {
            memory_system,
            thought_cloud,
            llm_processor: LlmProcessor::new(),
            externalization_threshold: 0.7, // Default threshold
            values,
            parallelism: ParallelismConfig::default(),
        }
    }
    
    /// Create a new thought pipeline with configuration
    pub fn new_with_config(
        memory_system: Arc<Mutex<MemorySystem>>, 
        thought_cloud: Arc<Mutex<ThoughtCloud>>,
        config: &BrainConfig
    ) -> Self {
        // Get values from config
        let values = config.values.core_values.clone();
        
        // Get parallelism config
        let parallelism = ParallelismConfig {
            enabled: config.pipeline.parallelism.enabled,
            thought_generation: config.pipeline.parallelism.thought_generation,
            base_instinct: config.pipeline.parallelism.base_instinct,
            values_assessment: config.pipeline.parallelism.values_assessment,
            psychological_processing: config.pipeline.parallelism.psychological_processing,
            externalization_decision: config.pipeline.parallelism.externalization_decision,
        };
        
        Self {
            memory_system,
            thought_cloud,
            llm_processor: LlmProcessor::new_with_config(&config.api),
            externalization_threshold: config.pipeline.externalization_threshold,
            values,
            parallelism,
        }
    }
    
    /// Get the core values
    pub fn values(&self) -> Vec<String> {
        self.values.clone()
    }
    
    /// Get the externalization threshold
    pub fn externalization_threshold(&self) -> f64 {
        self.externalization_threshold
    }
    
    /// Process the next thought in the pipeline
    pub async fn process_next_thought(&self) -> Result<Option<ProcessedThought>> {
        let start_time = Instant::now();
        info!("Beginning thought processing cycle");
        
        // Check if parallelism is enabled
        if self.parallelism.enabled {
            // Process multiple thoughts in parallel
            info!("Parallel thought processing enabled");
            self.process_parallel_thoughts().await?;
            
            // Return None since we're not tracking a single processed thought anymore
            let elapsed = start_time.elapsed();
            info!("Parallel thought processing cycle completed in {:?}", elapsed);
            return Ok(None);
        }
        
        // Log the current pipeline status for debugging
        self.log_pipeline_status().await?;
        
        // Single thought processing (legacy mode)
        // Check if there's a thought to process, with random chance of new thought generation
        let thought = {
            let mut cloud = self.thought_cloud.lock().await;
            
            // Add some randomness to thought generation
            use rand::{Rng, SeedableRng};
            use rand::rngs::StdRng;
            
            let now = std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap_or_default();
            
            let mut rng = StdRng::seed_from_u64(now.as_secs() + now.subsec_nanos() as u64);
            
            // Determine if we should generate a new thought even if the cloud isn't empty
            let force_new_thought = if !cloud.is_empty() {
                // 1 in 8 chance of generating a new thought even when the cloud isn't empty
                rng.gen_ratio(1, 8)
            } else {
                true // Always generate if the cloud is empty
            };
            
            if cloud.is_empty() || force_new_thought {
                if cloud.is_empty() {
                    info!("Thought cloud empty, generating new thought");
                } else {
                    info!("Randomly decided to generate a fresh thought for variety");
                }
                
                let seed_thought = self.generate_thoughts().await?;
                info!("Generated new thought: {}", seed_thought.content);
                cloud.add_thought(seed_thought.clone());
                seed_thought
            } else {
                // Get the next thought from the cloud
                let next_thought = cloud.next_thought().unwrap();
                info!("Processing existing thought: {}", next_thought.content);
                next_thought
            }
        };
        
        // Determine the current stage and process accordingly
        let stage = PipelineStage::from_str(&thought.pipeline_stage)
            .ok_or_else(|| anyhow!("Invalid pipeline stage: {}", thought.pipeline_stage))?;
            
        // Check if we've been stuck on this thought for too long
        let is_stuck = {
            // Extract the core thought content without stage markers
            let core_thought = thought.content.split(" (").next().unwrap_or(&thought.content);
            
            // Check if we've been stuck with this thought for multiple cycles
            let stuck_count = thought.metadata.get("stuck_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
                
            if stuck_count > 3 {
                info!("Detected stuck thought after {} cycles: {}", stuck_count, core_thought);
                true
            } else {
                false
            }
        };
        
        // Sometimes skip a stage to avoid getting stuck
        let stage_to_process = if is_stuck {
            // Jump ahead in the pipeline when stuck
            match stage {
                PipelineStage::ThoughtGeneration => PipelineStage::ValuesAssessment,
                PipelineStage::BaseInstinct => PipelineStage::PsychologicalProcessing,
                PipelineStage::ValuesAssessment => PipelineStage::ExternalizationDecision,
                PipelineStage::PsychologicalProcessing => PipelineStage::ExternalizationDecision,
                PipelineStage::ExternalizationDecision => PipelineStage::ExternalizationDecision, // Can't skip past the end
            }
        } else {
            stage
        };
        
        if stage_to_process != stage {
            info!("Skipping from {} to {} to avoid getting stuck", stage.as_str(), stage_to_process.as_str());
        }
        
        let processed_thought = match stage_to_process {
            PipelineStage::ThoughtGeneration => {
                // This should not happen, as ThoughtGeneration happens outside the main pipeline
                // But we'll handle it gracefully by moving to BaseInstinct
                info!("Processing thought generation (unexpected): {}", thought.content);
                ProcessedThought::new(thought.clone(), &thought.content)
            },
            PipelineStage::BaseInstinct => {
                self.process_base_instinct(&thought).await?
            },
            PipelineStage::ValuesAssessment => {
                self.process_values_assessment(&thought).await?
            },
            PipelineStage::PsychologicalProcessing => {
                self.process_psychological_structures(&thought).await?
            },
            PipelineStage::ExternalizationDecision => {
                self.process_externalization_decision(&thought).await?
            },
        };
        
        // Store or externalize the processed thought based on the current stage
        match stage_to_process {  // Use the potentially skipped stage
            PipelineStage::ThoughtGeneration | 
            PipelineStage::BaseInstinct | 
            PipelineStage::ValuesAssessment | 
            PipelineStage::PsychologicalProcessing => {
                // Move to the next stage
                if let Some(next_stage) = stage_to_process.next_stage() {
                    info!("Advancing thought from {} to {}", 
                          stage_to_process.as_str(), next_stage.as_str());
                    
                    let mut next_thought = thought.clone();
                    next_thought.pipeline_stage = next_stage.as_str().to_string();
                    next_thought.content = processed_thought.content.clone();
                    
                    // Store the processed metadata and increment stuck counter
                    let mut metadata = serde_json::from_value::<std::collections::HashMap<String, Value>>(
                        next_thought.metadata.clone()
                    ).unwrap_or_default();
                    
                    // Add tool results if there are any
                    if !processed_thought.tool_results.is_empty() {
                        info!("Storing tool results with thought");
                        metadata.insert("tool_results".to_string(), 
                            serde_json::to_value(&processed_thought.tool_results).unwrap_or_default());
                    }
                    
                    // Increment the stuck counter to track how long we've been processing this thought
                    let core_thought = next_thought.content.split(" (").next().unwrap_or(&next_thought.content).to_string();
                    let original_thought = thought.content.split(" (").next().unwrap_or(&thought.content).to_string();
                    
                    // Only increment if the core thought hasn't changed significantly
                    if core_thought.contains(&original_thought) || original_thought.contains(&core_thought) {
                        let stuck_count = metadata.get("stuck_count")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        
                        metadata.insert("stuck_count".to_string(), json!(stuck_count + 1));
                        
                        if stuck_count + 1 > 2 {
                            info!("This thought may be getting stuck (cycle {}): {}", stuck_count + 1, core_thought);
                        }
                    } else {
                        // Reset stuck counter as the thought has changed significantly
                        metadata.remove("stuck_count");
                    }
                    
                    // Update the metadata
                    next_thought.metadata = serde_json::to_value(metadata).unwrap_or_default();
                    
                    // Store in the thought cloud for the next stage
                    let mut cloud = self.thought_cloud.lock().await;
                    cloud.add_thought(next_thought.clone());
                    info!("Thought queued for next stage processing");
                    drop(cloud);
                    
                    // Update the existing thought in memory to the next stage
                    let mut memory = self.memory_system.lock().await;
                    
                    // First, check if we need to update an existing thought
                    if let Some(thought_id) = thought.id {
                        // Update the stage of the existing thought
                        if let Err(e) = memory.update_thought_stage(&thought_id, &next_stage.as_str()).await {
                            warn!("Failed to update thought stage in memory: {}", e);
                        } else {
                            info!("Updated thought stage in memory to: {}", next_stage.as_str());
                        }
                    } else {
                        // If somehow there's no ID, store as new thought
                        let memory_thought = memory::Thought {
                            id: next_thought.id,
                            content: next_thought.content.clone(),
                            thought_type: next_thought.thought_type.clone(),
                            pipeline_stage: next_thought.pipeline_stage.clone(),
                            metadata: next_thought.metadata.clone(),
                            created_at: next_thought.created_at,
                        };
                        
                        match memory.store_thought(&memory_thought).await {
                            Ok(id) => info!("Stored new advanced thought in memory with ID: {}", id),
                            Err(e) => warn!("Failed to store advanced thought in memory: {}", e),
                        };
                    }
                }
            },
            PipelineStage::ExternalizationDecision => {
                info!("Making externalization decision for thought");
                
                // Store the thought in memory regardless of externalization
                let mut memory = self.memory_system.lock().await;
                
                // Convert from brain_core::thought::Thought to memory::Thought
                let memory_thought = memory::Thought {
                    id: thought.id,
                    content: thought.content.clone(),
                    thought_type: thought.thought_type.clone(),
                    pipeline_stage: thought.pipeline_stage.clone(),
                    metadata: thought.metadata.clone(),
                    created_at: thought.created_at,
                };
                
                let id = memory.store_thought(&memory_thought).await?;
                info!("Thought stored in memory with ID: {}", id);
                
                // Decide whether to externalize the thought
                if processed_thought.externalization_score >= self.externalization_threshold {
                    info!("Externalization score {} exceeds threshold of {}, externalizing", 
                        processed_thought.externalization_score, self.externalization_threshold);
                    
                    // Mark as externalized in memory
                    memory.mark_thought_externalized(&id).await?;
                    
                    // Handle the actual externalization
                    self.externalize_thought(&processed_thought).await?;
                } else {
                    info!("Thought not externalized (score: {} below threshold: {}): {}", 
                        processed_thought.externalization_score, self.externalization_threshold, processed_thought.content);
                    
                    // Handle feedback loop - sometimes non-externalized thoughts 
                    // should be sent back for more processing
                    if processed_thought.externalization_score > 0.4 {
                        info!("Thought has potential (score > 0.4), creating derivative thought for further processing");
                        
                        // This thought has potential but needs refinement
                        // Create a derivative thought to send back to the beginning
                        let derivative_content = format!("Reflecting further on: {}", 
                            processed_thought.content.split(" (").next().unwrap_or(&processed_thought.content));
                        
                        let derivative_thought = Thought::new(&derivative_content);
                        info!("Created derivative thought: {}", derivative_content);
                        
                        // Add to thought cloud for further processing
                        let mut cloud = self.thought_cloud.lock().await;
                        cloud.add_thought(derivative_thought);
                        info!("Derivative thought added to thought cloud");
                    } else {
                        info!("Thought discarded without derivative (score <= 0.4)");
                    }
                }
            },
        }
        
        // Log processing time
        let elapsed = start_time.elapsed();
        info!("Thought processing cycle completed in {:?}", elapsed);
        
        Ok(Some(processed_thought))
    }

    /// Helper function to log pipeline status
    async fn log_pipeline_status(&self) -> Result<()> {
        let cloud = self.thought_cloud.lock().await;
        let thoughts = cloud.thoughts();
        
        // Count thoughts by stage
        let mut stage_counts = std::collections::HashMap::new();
        for stage in [
            PipelineStage::ThoughtGeneration,
            PipelineStage::BaseInstinct,
            PipelineStage::ValuesAssessment,
            PipelineStage::PsychologicalProcessing,
            PipelineStage::ExternalizationDecision,
        ] {
            let count = thoughts.iter()
                .filter(|t| PipelineStage::from_str(&t.pipeline_stage) == Some(stage))
                .count();
            stage_counts.insert(stage.as_str(), count);
        }
        
        info!("Pipeline status: ThoughtGeneration: {}, BaseInstinct: {}, ValuesAssessment: {}, PsychologicalProcessing: {}, ExternalizationDecision: {}",
            stage_counts.get("thought_generation").unwrap_or(&0),
            stage_counts.get("base_instinct").unwrap_or(&0),
            stage_counts.get("values_assessment").unwrap_or(&0),
            stage_counts.get("psychological_processing").unwrap_or(&0),
            stage_counts.get("externalization_decision").unwrap_or(&0)
        );
        
        Ok(())
    }
    
    /// Process multiple thoughts in parallel based on configuration
    async fn process_parallel_thoughts(&self) -> Result<()> {
        use tokio::task::JoinSet;
        use std::collections::HashMap;
        
        // Log current pipeline status
        self.log_pipeline_status().await?;
        
        // Collect thoughts to process by stage
        info!("Collecting thoughts to process in parallel");
        
        // Create a mapping of stage -> thoughts to process
        let thoughts_by_stage: HashMap<PipelineStage, Vec<Thought>> = {
            let mut cloud = self.thought_cloud.lock().await;
            let mut thoughts_map = HashMap::new();
            
            // Check if we need to generate new thoughts
            let thought_gen_count = self.parallelism.thought_generation;
            if thought_gen_count > 0 {
                // Always generate at least one new thought per cycle for variety
                info!("Generating {} new thoughts for variety", thought_gen_count);
                
                for _ in 0..thought_gen_count {
                    // Generate a fresh thought
                    match self.generate_thoughts().await {
                        Ok(mut thought) => {
                            info!("Generated new thought: {}", thought.content);
                            // Important: Set the pipeline_stage to BaseInstinct so it will be processed in the next cycle
                            thought.pipeline_stage = PipelineStage::BaseInstinct.as_str().to_string();
                            cloud.add_thought(thought);
                        },
                        Err(e) => {
                            warn!("Failed to generate thought: {}", e);
                        }
                    }
                }
            }
            
            // Group all thoughts in the cloud by stage
            for stage in &[
                PipelineStage::BaseInstinct,
                PipelineStage::ValuesAssessment, 
                PipelineStage::PsychologicalProcessing,
                PipelineStage::ExternalizationDecision
            ] {
                // Get thoughts for this stage
                let stage_thoughts: Vec<Thought> = cloud.thoughts()
                    .iter()
                    .filter(|t| PipelineStage::from_str(&t.pipeline_stage) == Some(*stage))
                    .cloned()
                    .collect();
                
                if !stage_thoughts.is_empty() {
                    thoughts_map.insert(*stage, stage_thoughts);
                }
            }
            
            // Remove the thoughts we've selected for processing from the cloud
            for (_, thoughts) in &thoughts_map {
                for thought in thoughts {
                    cloud.remove_thought(&thought.id);
                }
            }
            
            thoughts_map
        };
        
        // Create tasks for all stages, limiting by configured parallelism
        let mut all_tasks = JoinSet::new();
        
        // BaseInstinct stage
        if let Some(thoughts) = thoughts_by_stage.get(&PipelineStage::BaseInstinct) {
            info!("Processing {} thoughts in BaseInstinct stage", thoughts.len());
            
            // Process up to the configured limit
            let limit = self.parallelism.base_instinct;
            for thought in thoughts.iter().take(limit) {
                let pipe_clone = self.clone();
                let thought_clone = thought.clone();
                
                all_tasks.spawn(async move {
                    let result = pipe_clone.process_base_instinct(&thought_clone).await;
                    (PipelineStage::BaseInstinct, thought_clone, result)
                });
            }
        }
        
        // ValuesAssessment stage
        if let Some(thoughts) = thoughts_by_stage.get(&PipelineStage::ValuesAssessment) {
            info!("Processing {} thoughts in ValuesAssessment stage", thoughts.len());
            
            // Process up to the configured limit
            let limit = self.parallelism.values_assessment;
            for thought in thoughts.iter().take(limit) {
                let pipe_clone = self.clone();
                let thought_clone = thought.clone();
                
                all_tasks.spawn(async move {
                    let result = pipe_clone.process_values_assessment(&thought_clone).await;
                    (PipelineStage::ValuesAssessment, thought_clone, result)
                });
            }
        }
        
        // PsychologicalProcessing stage
        if let Some(thoughts) = thoughts_by_stage.get(&PipelineStage::PsychologicalProcessing) {
            info!("Processing {} thoughts in PsychologicalProcessing stage", thoughts.len());
            
            // Process up to the configured limit
            let limit = self.parallelism.psychological_processing;
            for thought in thoughts.iter().take(limit) {
                let pipe_clone = self.clone();
                let thought_clone = thought.clone();
                
                all_tasks.spawn(async move {
                    let result = pipe_clone.process_psychological_structures(&thought_clone).await;
                    (PipelineStage::PsychologicalProcessing, thought_clone, result)
                });
            }
        }
        
        // ExternalizationDecision stage
        if let Some(thoughts) = thoughts_by_stage.get(&PipelineStage::ExternalizationDecision) {
            info!("Processing {} thoughts in ExternalizationDecision stage", thoughts.len());
            
            // Process up to the configured limit
            let limit = self.parallelism.externalization_decision;
            for thought in thoughts.iter().take(limit) {
                let pipe_clone = self.clone();
                let thought_clone = thought.clone();
                
                all_tasks.spawn(async move {
                    let result = pipe_clone.process_externalization_decision(&thought_clone).await;
                    (PipelineStage::ExternalizationDecision, thought_clone, result)
                });
            }
        }
        
        // Wait for all tasks to complete and handle the results
        let mut completed_count = 0;
        while let Some(result) = all_tasks.join_next().await {
            match result {
                Ok((stage, thought, process_result)) => {
                    match process_result {
                        Ok(processed_thought) => {
                            completed_count += 1;
                            info!("Completed processing thought in {} stage: {}", 
                                  stage.as_str(), processed_thought.content);
                            
                            // Handle the processed thought based on its stage
                            match stage {
                                PipelineStage::ThoughtGeneration | 
                                PipelineStage::BaseInstinct | 
                                PipelineStage::ValuesAssessment | 
                                PipelineStage::PsychologicalProcessing => {
                                    // Move to the next stage
                                    if let Some(next_stage) = stage.next_stage() {
                                        info!("Advancing thought from {} to {}", 
                                              stage.as_str(), next_stage.as_str());
                                        
                                        let mut next_thought = thought.clone();
                                        next_thought.pipeline_stage = next_stage.as_str().to_string();
                                        next_thought.content = processed_thought.content.clone();
                                        
                                        // Store the processed metadata and increment stuck counter
                                        let mut metadata = serde_json::from_value::<std::collections::HashMap<String, Value>>(
                                            next_thought.metadata.clone()
                                        ).unwrap_or_default();
                                        
                                        // Add tool results if there are any
                                        if !processed_thought.tool_results.is_empty() {
                                            info!("Storing tool results with thought");
                                            metadata.insert("tool_results".to_string(), 
                                                serde_json::to_value(&processed_thought.tool_results).unwrap_or_default());
                                        }
                                        
                                        // Increment the stuck counter to track how long we've been processing this thought
                                        let core_thought = next_thought.content.split(" (").next()
                                            .unwrap_or(&next_thought.content).to_string();
                                        let original_thought = thought.content.split(" (").next()
                                            .unwrap_or(&thought.content).to_string();
                                        
                                        // Only increment if the core thought hasn't changed significantly
                                        if core_thought.contains(&original_thought) || original_thought.contains(&core_thought) {
                                            let stuck_count = metadata.get("stuck_count")
                                                .and_then(|v| v.as_u64())
                                                .unwrap_or(0);
                                            
                                            metadata.insert("stuck_count".to_string(), json!(stuck_count + 1));
                                            
                                            if stuck_count + 1 > 2 {
                                                info!("This thought may be getting stuck (cycle {}): {}", 
                                                      stuck_count + 1, core_thought);
                                            }
                                        } else {
                                            // Reset stuck counter as the thought has changed significantly
                                            metadata.remove("stuck_count");
                                        }
                                        
                                        // Update the metadata
                                        next_thought.metadata = serde_json::to_value(metadata).unwrap_or_default();
                                        
                                        // Store in the thought cloud for the next stage
                                        let mut cloud = self.thought_cloud.lock().await;
                                        cloud.add_thought(next_thought);
                                        info!("Thought queued for next stage processing");
                                    }
                                },
                                PipelineStage::ExternalizationDecision => {
                                    info!("Making externalization decision for thought");
                                    
                                    // Store the thought in memory regardless of externalization
                                    let mut memory = self.memory_system.lock().await;
                                    
                                    // Convert from brain_core::thought::Thought to memory::Thought
                                    let memory_thought = memory::Thought {
                                        id: thought.id,
                                        content: thought.content.clone(),
                                        thought_type: thought.thought_type.clone(),
                                        pipeline_stage: thought.pipeline_stage.clone(),
                                        metadata: thought.metadata.clone(),
                                        created_at: thought.created_at,
                                    };
                                    
                                    let id = memory.store_thought(&memory_thought).await?;
                                    info!("Thought stored in memory with ID: {}", id);
                                    
                                    // Decide whether to externalize the thought
                                    if processed_thought.externalization_score >= self.externalization_threshold {
                                        info!("Externalization score {} exceeds threshold of {}, externalizing", 
                                            processed_thought.externalization_score, self.externalization_threshold);
                                        
                                        // Mark as externalized in memory
                                        memory.mark_thought_externalized(&id).await?;
                                        
                                        // Handle the actual externalization
                                        self.externalize_thought(&processed_thought).await?;
                                    } else {
                                        info!("Thought not externalized (score: {} below threshold: {}): {}", 
                                            processed_thought.externalization_score, self.externalization_threshold, 
                                            processed_thought.content);
                                        
                                        // Handle feedback loop - sometimes non-externalized thoughts 
                                        // should be sent back for more processing
                                        if processed_thought.externalization_score > 0.4 {
                                            info!("Thought has potential (score > 0.4), creating derivative thought for further processing");
                                            
                                            // This thought has potential but needs refinement
                                            // Create a derivative thought to send back to the beginning
                                            let derivative_content = format!("Reflecting further on: {}", 
                                                processed_thought.content.split(" (").next().unwrap_or(&processed_thought.content));
                                            
                                            let derivative_thought = Thought::new(&derivative_content);
                                            info!("Created derivative thought: {}", derivative_content);
                                            
                                            // Add to thought cloud for further processing
                                            let mut cloud = self.thought_cloud.lock().await;
                                            cloud.add_thought(derivative_thought);
                                            info!("Derivative thought added to thought cloud");
                                        } else {
                                            info!("Thought discarded without derivative (score <= 0.4)");
                                        }
                                    }
                                },
                            }
                        },
                        Err(e) => {
                            warn!("Error processing thought in {} stage: {}", stage.as_str(), e);
                            
                            // Put the thought back in the cloud for future processing
                            let mut cloud = self.thought_cloud.lock().await;
                            cloud.add_thought(thought);
                        }
                    }
                },
                Err(e) => {
                    warn!("Task joined with error: {}", e);
                }
            }
        }
        
        info!("Completed parallel processing of {} thoughts", completed_count);
        
        // Log final pipeline status after processing
        self.log_pipeline_status().await?;
        
        Ok(())
    }
    
    /// Generate thoughts to start the pipeline
    async fn generate_thoughts(&self) -> Result<Thought> {
        info!("Generating new thoughts");
        
        // Get relevant context from memory
        let memory_system = self.memory_system.lock().await;
        let recent_thoughts = memory_system.get_recent_thoughts(10).await?;
        let important_memories = memory_system.get_important_memories(5).await?;
        
        // Add a timestamp to make each prompt unique
        let timestamp = chrono::Utc::now().timestamp();
        
        // Format context for the LLM
        let context = if recent_thoughts.is_empty() && important_memories.is_empty() {
            info!("No existing context found, using seed context");
            "Starting fresh with no previous context.".to_string()
        } else {
            format!(
                "Recent thoughts:\n{}\n\nImportant memories:\n{}", 
                recent_thoughts.iter()
                    .map(|t| format!("- {}", t.content))
                    .collect::<Vec<_>>()
                    .join("\n"),
                important_memories.iter()
                    .map(|m| format!("- {} (importance: {})", m.content, m.importance))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        };
        
        info!("Context for LLM: {}", context);
        
        // Add some randomness to the prompt
        let thought_starters = [
            "I wonder...",
            "I should...",
            "I need to...",
            "I could...",
            "Perhaps I should...",
        ];
        
        use rand::{Rng, SeedableRng};
        use rand::rngs::StdRng;
        let seed = timestamp as u64;
        let mut rng = StdRng::seed_from_u64(seed);
        let starter = thought_starters[rng.gen_range(0..thought_starters.len())];
        
        // Modified prompt with randomness
        let prompt = format!(
            "Generate a concise, helpful thought that an AI assistant might have while considering how best to assist users.\n\
            \n\
            Context (recent information):\n{}\n\
            \n\
            Core values to consider:\n{}\n\
            \n\
            Instructions:\n\
            1. Create a single sentence thought that starts with \"{}\"\n\
            2. The thought should be about helping users or improving assistance\n\
            3. Keep it concise and specific (15 words or less)\n\
            4. Make it unique and different from previous thoughts\n\
            5. No explanations or meta-commentary - just the thought itself\n\
            \n\
            Current time: {}\n\
            Generate just the thought, starting with \"I\":",
            context,
            self.values.iter()
                .map(|v| format!("- {}", v))
                .collect::<Vec<_>>()
                .join("\n"),
            starter,
            chrono::Utc::now().format("%H:%M:%S")
        );
        
        // Use LLM to generate a thought or fall back to templates
        let content = match self.llm_processor.process_thought(&Thought::new("Generate thought"), &prompt).await {
            Ok(thought_content) => {
                info!("Successfully generated thought with LLM: {}", thought_content.trim());
                thought_content.trim().to_string()
            },
            Err(e) => {
                warn!("Error generating thought with LLM: {}", e);
                info!("Falling back to template system");
                
                // A much larger list of possible thought templates for variety
                let thought_templates = [
                    // User-focused thoughts
                    "I wonder what the user might need right now",
                    "I should anticipate the user's next question about this topic",
                    "I should consider what information might be most valuable to the user",
                    "I need to remember the user's previous requests to provide better context",
                    "I wonder if the user would appreciate a more detailed explanation",
                    "I should check if there are any tasks the user might need help with",
                    
                    // Self-improvement thoughts
                    "I should refine my understanding of technical concepts to better assist users",
                    "I need to organize information more effectively for clarity",
                    "I wonder if there are better ways to explain complex concepts",
                    "I should explore more concise ways to present information",
                    "I could improve my pattern recognition for common user needs",
                    
                    // Reflective thoughts
                    "I need to evaluate if my responses have been helpful",
                    "I should reflect on what approaches have worked well recently",
                    "I wonder if there are patterns in the questions I've been asked",
                    "I should consider how to provide more value in my responses",
                    "I need to balance thoroughness with conciseness in my explanations",
                    
                    // Action-oriented thoughts
                    "I should suggest specific next steps when solving problems",
                    "I need to gather more relevant context before responding",
                    "I should verify facts before presenting them as certain",
                    "I wonder if there are resources I could recommend on this topic",
                    "I should break down complex tasks into manageable steps"
                ];
                
                // Choose a random thought template with added randomness in selection
                use std::time::SystemTime;
                use rand::{Rng, SeedableRng};
                use rand::rngs::StdRng;
                
                let seed_value = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .subsec_nanos();
                    
                let mut rng = StdRng::seed_from_u64(seed_value as u64);
                let template_index = rng.gen_range(0..thought_templates.len());
                let template = thought_templates[template_index];
                
                info!("Selected template: {}", template);
                
                // Sometimes add modifiers to make thoughts more unique
                let modifiers = ["specifically", "carefully", "thoughtfully", "proactively", "systematically"];
                let should_add_modifier = rng.gen_ratio(1, 3); // 1 in 3 chance
                
                if should_add_modifier && template.contains("should") {
                    let modifier = modifiers[rng.gen_range(0..modifiers.len())];
                    // Insert the modifier after "should"
                    let parts: Vec<&str> = template.splitn(2, "should").collect();
                    if parts.len() == 2 {
                        format!("I should {} {} ({})", modifier, parts[1].trim(), chrono::Utc::now().format("%H:%M:%S"))
                    } else {
                        format!("{} ({})", template, chrono::Utc::now().format("%H:%M:%S"))
                    }
                } else {
                    format!("{} ({})", template, chrono::Utc::now().format("%H:%M:%S"))
                }
            }
        };
        
        // Create a new thought at the base instinct stage
        // This ensures the thought immediately enters the processing pipeline
        let mut thought = Thought::new(&content);
        thought.pipeline_stage = PipelineStage::BaseInstinct.as_str().to_string();
        
        // Store the thought in memory immediately
        info!("Storing generated thought in memory");
        let memory_thought = memory::Thought {
            id: thought.id.clone(),
            content: thought.content.clone(),
            thought_type: thought.thought_type.clone(),
            pipeline_stage: thought.pipeline_stage.clone(),
            metadata: thought.metadata.clone(),
            created_at: thought.created_at,
        };
        
        // Need to drop the previous lock before acquiring a new one
        drop(memory_system);
        
        let mut memory_system = self.memory_system.lock().await;
        match memory_system.store_thought(&memory_thought).await {
            Ok(id) => {
                info!("Stored thought in memory with ID: {}", id);
            },
            Err(e) => {
                warn!("Failed to store thought in memory: {}", e);
            }
        }
        
        Ok(thought)
    }
    
    /// Process a thought at the base instinct stage
    async fn process_base_instinct(&self, thought: &Thought) -> Result<ProcessedThought> {
        debug!("Processing base instinct: {}", thought.content);
        
        // Process the thought through the base instincts layer
        // This addresses simple questions about environment, stimuli, and past experiences
        
        // Prepare tools for this stage
        let tools = vec![
            LlmProcessor::get_context_tool(), // For environment awareness
            LlmProcessor::memory_search_tool(), // For accessing past experiences
        ];
        
        // Create a simpler prompt to avoid Claude rejecting the roleplay
        let prompt = format!(
            "Consider the following thought and refine it to be more specific, practical, and context-aware:\n\
            \n\
            Original thought: \"{}\"\n\
            \n\
            Consider these factors:\n\
            1. Current environment and immediate context\n\
            2. Relevant past experiences or similar situations\n\
            3. Basic needs that this thought is addressing\n\
            4. How this might benefit both the assistant and user\n\
            \n\
            You can use tools to gather more information if needed.\n\
            \n\
            Provide the refined thought (starting with \"I\"):",
            thought.content
        );
        
        // Process the thought with the LLM and tools
        let (processed_content, tool_results) = match self.llm_processor.process_thought_with_tools(thought, &prompt, tools).await {
            Ok((content, results)) => (content, results),
            Err(e) => {
                warn!("Error in base instinct processing: {}", e);
                // Fall back to simple processing
                (format!("{} (processed through base instincts)", thought.content), Vec::new())
            }
        };
        
        // Create the processed thought
        let processed = ProcessedThought::new(thought.clone(), &processed_content)
            .with_tool_results(tool_results);
        
        Ok(processed)
    }
    
    /// Process a thought at the values assessment stage using true/false evaluation
    async fn process_values_assessment(&self, thought: &Thought) -> Result<ProcessedThought> {
        info!("Processing values assessment: {}", thought.content);
        
        // Get active goals
        let mut memory_system = self.memory_system.lock().await;
        let active_goals = memory_system.get_active_goals().await?;
        
        // Format goals for the prompt
        let goals_formatted = active_goals.iter()
            .map(|g| format!("Goal {}: \"{}\" (Priority: {}, Progress: {:.1}%)", 
                g.id, g.description, g.priority, g.progress * 100.0))
            .collect::<Vec<_>>()
            .join("\n");
        
        let prompt = format!(
            "I need to evaluate if my current thought aligns with my values and active goals.\n\
            \n\
            Current thought: \"{}\"\n\
            \n\
            Active Goals:\n{}\n\
            \n\
            Core Values:\n{}\n\
            \n\
            Instructions:\n\
            1. Evaluate how this thought aligns with our values\n\
            2. Determine if this thought contributes to any active goals\n\
            3. Consider if we should adjust the thought to better serve our goals\n\
            \n\
            Respond in JSON format:\n\
            {{\n\
              \"values_alignment\": 0.0-1.0,\n\
              \"related_goals\": [\"goal_ids\"],\n\
              \"goal_contribution\": 0.0-1.0,\n\
              \"revised_thought\": \"Updated thought if needed\",\n\
              \"explanation\": \"Brief explanation of assessment\"\n\
            }}",
            thought.content,
            goals_formatted,
            self.values.iter()
                .map(|v| format!("- {}", v))
                .collect::<Vec<_>>()
                .join("\n")
        );
        
        // Process the thought with the LLM
        let evaluation_result = match self.llm_processor.process_thought(thought, &prompt).await {
            Ok(json_string) => {
                // Try to parse the JSON response
                match serde_json::from_str::<Value>(&json_string) {
                    Ok(json) => {
                        // Extract the key information from the evaluation
                        let values_alignment = json.get("values_alignment")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0);
                        
                        let goal_contribution = json.get("goal_contribution")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0);
                        
                        // Get the revised thought or use the original
                        let processed_content = if values_alignment < 0.3 {
                            json.get("revised_thought")
                                .and_then(|v| v.as_str())
                                .unwrap_or(&thought.content)
                                .to_string()
                        } else {
                                // If no revision required, keep the original thought
                            // But check if there are any goal-related insights
                            // Create an empty array as a fallback value with proper lifetime
                            let empty_array = serde_json::json!([]);
                            let empty_array_ref = empty_array.as_array().unwrap();
                            
                            let related_goals = json.get("related_goals")
                                .and_then(|v| v.as_array())
                                .unwrap_or(empty_array_ref);
                                
                            if !related_goals.is_empty() {
                                // Log that we found related goals
                                info!("Thought relates to goals: {:?}", related_goals);
                                
                                // We should link this thought to the related goals
                                for goal_id_json in related_goals {
                                    if let Some(goal_id) = goal_id_json.as_str() {
                                        if let Some(t_id) = thought.id {
                                            // Drop memory system lock and reacquire it to prevent deadlock
                                            drop(memory_system);
                                            let mut memory = self.memory_system.lock().await;
                                            if let Err(e) = memory.link_thought_to_goal(goal_id, &t_id).await {
                                                warn!("Failed to link thought to goal {}: {}", goal_id, e);
                                            } else {
                                                info!("Linked thought to goal {}", goal_id);
                                            }
                                            drop(memory);
                                            // Reacquire the lock for the rest of the function
                                            memory_system = self.memory_system.lock().await;
                                        }
                                    }
                                }
                            }
                            
                            thought.content.clone()
                        };
                        
                        // Log the values assessment
                        info!(
                            "Values assessment complete - Values alignment: {}%, Goal contribution: {}", 
                            (values_alignment * 100.0).round() / 100.0,
                            (goal_contribution * 100.0).round() / 100.0
                        );
                        
                        if values_alignment < 0.3 {
                            info!("Original thought conflicted with values, revised to: {}", processed_content);
                        } else {
                            info!("Thought aligns with values, no revision needed");
                        }
                        
                        (processed_content, json)
                    },
                    Err(e) => {
                        warn!("Error parsing values assessment JSON: {}", e);
                        // Fall back to original thought in case of error
                        (thought.content.clone(), json!({}))
                    }
                }
            },
            Err(e) => {
                warn!("Error in values assessment: {}", e);
                // Fall back to original thought
                (thought.content.clone(), json!({}))
            }
        };
        
        let (processed_content, metadata) = evaluation_result;
        
        // Create the processed thought
        let mut processed = ProcessedThought::new(thought.clone(), &processed_content);
        
        // Add the values assessment metadata
        processed.add_metadata("values_assessment", metadata);
        
        Ok(processed)
    }
    
    /// Process a thought through psychological structures
    async fn process_psychological_structures(&self, thought: &Thought) -> Result<ProcessedThought> {
        info!("Processing psychological structures: {}", thought.content);
        
        // For now, we'll skip psychological processing since Claude is objecting to the roleplay
        // Instead, we'll use the original thought content but add a simple tag
        
        // Extract original thought content that might be hiding in JSON
        let original_thought = if thought.content.starts_with("{") {
            // If this is a JSON string, extract the meaningful part
            // This is a temporary fix until we resolve the Claude roleplay issue
            "I wonder what the user might need right now".to_string()
        } else {
            thought.content.clone()
        };
        
        info!("Using original thought: {}", original_thought);
        
        // Create a processed thought
        let processed_content = format!("{} (psychologically processed)", original_thought);
        
        // Log that we're skipping the full psychological processing
        info!("Skipping full psychological structure processing (Claude is rejecting the roleplay)");
        
        
        // Add metadata about the psychological processing
        let mut metadata = serde_json::from_value::<std::collections::HashMap<String, Value>>(
            thought.metadata.clone()
        ).unwrap_or_default();
        
        metadata.insert("psychological_processing".to_string(), 
            serde_json::json!(true));
        
        // Create the processed thought with the final result
        let mut processed_thought = Thought::new(&processed_content);
        processed_thought.metadata = serde_json::to_value(metadata).unwrap_or_default();
        
        let processed = ProcessedThought::new(processed_thought, &processed_content);
        
        Ok(processed)
    }
    
    /// Process a thought at the externalization decision stage
    async fn process_externalization_decision(&self, thought: &Thought) -> Result<ProcessedThought> {
        info!("Processing externalization decision: {}", thought.content);
        
        // Get existing externalized thoughts to avoid duplication
        let memory_system = self.memory_system.lock().await;
        let recent_thoughts = memory_system.get_recent_thoughts(20).await?;
        
        // Extract the core thought content (removing any JSON or processing markers)
        let content_without_markers = if thought.content.starts_with("{") {
            // If this is a JSON response, use a default thought
            "I should focus on understanding the user's needs and providing helpful information".to_string()
        } else {
            // Otherwise extract the core content without timestamps/markers
            thought.content.split(" (").next().unwrap_or(&thought.content).to_string()
        };
        
        info!("Using core thought content: {}", content_without_markers);
        
        // Check if this thought has already been externalized
        let already_externalized = recent_thoughts.iter()
            .filter(|t| t.pipeline_stage == "externalization_decision")
            .any(|t| {
                // Extract content without timestamp for comparison
                let t_content = t.content.split(" (").next().unwrap_or(&t.content);
                let metadata = &t.metadata;
                
                // Check if the thought is similar and has been externalized
                let is_externalized = metadata.get("externalized")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                
                t_content.contains(&content_without_markers) && is_externalized
            });
        
        if already_externalized {
            info!("This thought is similar to one already externalized, will not externalize again");
        } else {
            info!("This thought has not been externalized before");
        }
        
        // Set an appropriate externalization score
        let score = if already_externalized {
            0.3 // Below threshold, will not externalize again
        } else {
            0.85 // Above threshold, will externalize this thought
        };
        
        // Create a refined version of the thought for externalization
        let refined_thought = format!("{}", content_without_markers);
        
        // Determine if an action is needed (for now, no actions)
        let action: Option<String> = None;
        
        // Skip the LLM call since it's rejecting the roleplay
        info!("Externalization decision made: score={}, will_externalize={}", 
            score, score >= self.externalization_threshold);
        
        // Create a properly formatted decision
        let decision = (score, action, refined_thought);
        
        let (score, action, content) = decision;
        
        info!("Externalization score for '{}': {}", content_without_markers, score);
        
        // Create the processed thought with the decision
        let processed = ProcessedThought::new(thought.clone(), &content)
            .with_externalization_score(score);
        
        // Add the suggested action if available
        let processed = if let Some(action_str) = action {
            processed.with_suggested_action(&action_str)
        } else {
            processed
        };
        
        Ok(processed)
    }
    
    /// Externalize a thought (either through action or vocalization)
    async fn externalize_thought(&self, thought: &ProcessedThought) -> Result<()> {
        info!("Externalizing thought: {}", thought.content);
        
        // Store the thought as a chat message if it's suitable for communication
        let mut memory_system = self.memory_system.lock().await;
        
        // Get active goals to check relevance
        let active_goals = memory_system.get_active_goals().await?;
        
        if let Some(action) = &thought.suggested_action {
            info!("Taking action: {}", action);
            info!("Based on thought: {}", thought.content);
            // Execute the action
            // In a complete implementation, this would invoke the appropriate tool
            
            // If the action involves communicating with the user, format it as a message
            if action.contains("tell") || action.contains("ask") || action.contains("inform") {
                let message = format!("{} (Based on: {})", action, thought.content);
                memory_system.store_chat_message(memory::ChatSender::Brain, &message).await?;
            }
        } else {
            // Check if the thought is suitable for externalization as a chat message
            // Look for thoughts that seem like they should be communicated
            let content = &thought.content;
            
            // Base communication criteria
            let is_communicative = 
                content.contains("I should tell") || 
                content.contains("I should ask") || 
                content.contains("I wonder if the user") ||
                content.contains("I should explain") ||
                content.contains("I could help") ||
                thought.externalization_score > 0.85; // High confidence thoughts
            
            // Check if thought is related to any high-priority goal
            let is_goal_related = if !active_goals.is_empty() {
                // Check for goal-related content 
                let high_priority_goals = active_goals.iter()
                    .filter(|g| g.priority >= 8)
                    .collect::<Vec<_>>();
                
                // If we have high priority goals, check if the thought relates to them
                if !high_priority_goals.is_empty() {
                    let content_lower = content.to_lowercase();
                    
                    high_priority_goals.iter().any(|g| {
                        // Extract key terms from goal description
                        let goal_desc_lower = g.description.to_lowercase();
                        let goal_terms = goal_desc_lower.split_whitespace()
                            .filter(|term| term.len() > 3) // Only significant words
                            .collect::<Vec<_>>();
                        
                        // Check if thought contains any of these terms
                        goal_terms.iter().any(|term| content_lower.contains(term))
                    })
                } else {
                    false
                }
            } else {
                false
            };
                
            // Determine if we should communicate based on either criteria
            if is_communicative || is_goal_related {
                // Format the thought as a chat message from the brain
                let formatted_message = if content.starts_with("I ") {
                    // Convert from first-person to second-person
                    content
                        .replace("I should tell the user ", "")
                        .replace("I should ask the user ", "")
                        .replace("I should explain ", "")
                        .replace("I wonder if ", "I'm wondering if ")
                        .replace("I could help ", "I can help ")
                } else {
                    content.clone()
                };
                
                // Store as a chat message
                memory_system.store_chat_message(memory::ChatSender::Brain, &formatted_message).await?;
                info!("Sent thought to chat: {}", formatted_message);
                
                // If this thought relates to a goal, link it to that goal
                if is_goal_related {
                    // Find the goal(s) it relates to
                    for goal in &active_goals {
                        let content_lower = content.to_lowercase();
                        let goal_desc_lower = goal.description.to_lowercase();
                        let goal_terms = goal_desc_lower.split_whitespace()
                            .filter(|term| term.len() > 3)
                            .collect::<Vec<_>>();
                        
                        if goal_terms.iter().any(|term| content_lower.contains(term)) {
                            // Link the original thought to this goal
                            if let Some(thought_id) = thought.original_thought.id {
                                memory_system.link_thought_to_goal(&goal.id, &thought_id).await?;
                                info!("Linked thought to goal: {}", goal.description);
                            }
                        }
                    }
                }
            } else {
                info!("Thought was not suitable for chat communication: {}", content);
            }
        }
        
        Ok(())
    }

    pub async fn generate_or_update_goals(&self) -> Result<()> {
        let memory_system = self.memory_system.lock().await;
        let recent_thoughts = memory_system.get_recent_thoughts(20).await?;
        let active_goals = memory_system.get_active_goals().await?;
        
        // Prepare the prompt to analyze thoughts and goals
        let prompt = format!(
            "Based on recent thoughts and current goals, determine if we need new goals or should update existing ones.\n\
            \n\
            Recent Thoughts:\n{}\n\
            \n\
            Current Goals:\n{}\n\
            \n\
            Instructions:\n\
            1. Analyze patterns in recent thoughts\n\
            2. Identify any emerging objectives\n\
            3. Suggest new goals or updates to existing ones\n\
            \n\
            Respond in JSON format with this structure:\n\
            {{\n\
              \"new_goals\": [\n\
                {{ \"description\": \"...\", \"priority\": 1-10 }}\n\
              ],\n\
              \"goal_updates\": [\n\
                {{ \"id\": \"goal_id\", \"progress\": 0.0-1.0, \"status\": \"Active|Completed|Abandoned|Blocked\" }}\n\
              ]\n\
            }}",
            recent_thoughts.iter()
                .map(|t| format!("- {}", t.content))
                .collect::<Vec<_>>()
                .join("\n"),
            active_goals.iter()
                .map(|g| format!("- {} (ID: {}, Priority: {}, Progress: {:.1}%)", 
                    g.description, g.id, g.priority, g.progress * 100.0))
                .collect::<Vec<_>>()
                .join("\n")
        );

        // Release the memory system lock before LLM processing
        drop(memory_system);
        
        // Process with LLM
        match self.llm_processor.generate_text(&prompt).await {
            Ok(response) => {
                // Try to parse the JSON response
                match serde_json::from_str::<Value>(&response) {
                    Ok(json) => {
                        // Reacquire the lock to update goals
                        let mut memory_system = self.memory_system.lock().await;
                        
                        // Process new goals
                        if let Some(new_goals) = json.get("new_goals").and_then(|g| g.as_array()) {
                            for goal_json in new_goals {
                                if let (Some(description), Some(priority)) = (
                                    goal_json.get("description").and_then(|d| d.as_str()),
                                    goal_json.get("priority").and_then(|p| p.as_u64()).map(|p| p as u8)
                                ) {
                                    // Create a new goal from the suggested description and priority
                                    let new_goal = memory::Goal {
                                        id: uuid::Uuid::new_v4().to_string(),
                                        description: description.to_string(),
                                        priority: priority.clamp(1, 10),
                                        status: memory::GoalStatus::Active,
                                        created_at: chrono::Utc::now(),
                                        completed_at: None,
                                        related_thoughts: Vec::new(),
                                        progress: 0.0,
                                        parent_goal: None,
                                        sub_goals: Vec::new(),
                                    };
                                    
                                    // Add the goal to memory
                                    if let Err(e) = memory_system.add_goal(new_goal.clone()).await {
                                        warn!("Failed to add new goal '{}': {}", description, e);
                                    } else {
                                        info!("Added new goal: {} (Priority: {})", description, priority);
                                    }
                                }
                            }
                        }
                        
                        // Process goal updates
                        if let Some(goal_updates) = json.get("goal_updates").and_then(|g| g.as_array()) {
                            for update in goal_updates {
                                if let Some(goal_id) = update.get("id").and_then(|id| id.as_str()) {
                                    // Update progress if provided
                                    if let Some(progress) = update.get("progress").and_then(|p| p.as_f64()) {
                                        let progress = progress as f32;
                                        if progress >= 0.0 && progress <= 1.0 {
                                            if let Err(e) = memory_system.update_goal_progress(goal_id, progress).await {
                                                warn!("Failed to update goal progress: {}", e);
                                            } else {
                                                info!("Updated goal {} progress to {:.1}%", goal_id, progress * 100.0);
                                            }
                                        }
                                    }
                                    
                                    // Update status if provided
                                    if let Some(status_str) = update.get("status").and_then(|s| s.as_str()) {
                                        let status = match status_str {
                                            "Active" => Some(memory::GoalStatus::Active),
                                            "Completed" => Some(memory::GoalStatus::Completed),
                                            "Abandoned" => Some(memory::GoalStatus::Abandoned),
                                            "Blocked" => Some(memory::GoalStatus::Blocked),
                                            _ => None,
                                        };
                                        
                                        if let Some(status_value) = status {
                                            if let Err(e) = memory_system.update_goal_status(goal_id, status_value.clone()).await {
                                                warn!("Failed to update goal status: {}", e);
                                            } else {
                                                info!("Updated goal {} status to {:?}", goal_id, status_value);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    Err(e) => {
                        warn!("Failed to parse goal generation response as JSON: {}", e);
                    }
                }
            },
            Err(e) => {
                warn!("Failed to generate goal suggestions: {}", e);
            }
        }
        
        Ok(())
    }
}