//! G-Code processor pipeline and registry

use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::{GcodeCommand, GcodeState};

/// Configuration options for command processors
///
/// Provides customizable settings for different preprocessor implementations.
/// Can be extended by specific processor implementations for their unique needs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessorConfig {
    /// Whether this processor is enabled
    pub enabled: bool,
    /// Processor-specific configuration data (JSON-like)
    pub options: std::collections::HashMap<String, String>,
}

impl ProcessorConfig {
    /// Create a new processor configuration
    pub fn new() -> Self {
        Self {
            enabled: true,
            options: std::collections::HashMap::new(),
        }
    }

    /// Create a disabled processor configuration
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            options: std::collections::HashMap::new(),
        }
    }

    /// Set a configuration option
    pub fn with_option(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.options.insert(key.into(), value.into());
        self
    }

    /// Get a configuration option
    pub fn get_option(&self, key: &str) -> Option<&str> {
        self.options.get(key).map(|s| s.as_str())
    }
}

impl Default for ProcessorConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for G-Code command processors
///
/// Processors implement transformations, validations, and modifications
/// to G-Code commands. They are applied in a pipeline to process commands
/// before execution.
///
/// # Examples
/// - Comment removal
/// - Whitespace normalization
/// - Arc expansion to line segments
/// - Command optimization
/// - Feed rate overrides
pub trait CommandProcessor: Send + Sync {
    /// Get the name/identifier of this processor
    fn name(&self) -> &str;

    /// Get a description of what this processor does
    fn description(&self) -> &str;

    /// Process a single G-Code command
    ///
    /// # Arguments
    /// * `command` - The G-Code command to process
    /// * `state` - Current G-Code state (modal state)
    ///
    /// # Returns
    /// A vector of processed commands. Most processors return a single command,
    /// but some (like arc expanders) may expand one command into many.
    /// Return an empty vector to skip the command.
    fn process(
        &self,
        command: &GcodeCommand,
        state: &GcodeState,
    ) -> Result<Vec<GcodeCommand>, String>;

    /// Check if this processor is enabled
    fn is_enabled(&self) -> bool {
        true
    }

    /// Get the configuration for this processor
    fn config(&self) -> &ProcessorConfig {
        static DEFAULT_CONFIG: std::sync::OnceLock<ProcessorConfig> = std::sync::OnceLock::new();
        DEFAULT_CONFIG.get_or_init(ProcessorConfig::new)
    }
}

/// Arc-wrapped processor for thread-safe sharing
pub type ProcessorHandle = Arc<dyn CommandProcessor>;

/// G-Code command processor pipeline
///
/// Manages a sequence of command processors that are applied to G-Code commands
/// in order. Each processor can transform the command, skip it, or expand it
/// into multiple commands.
///
/// # Example
/// ```ignore
/// let mut pipeline = ProcessorPipeline::new();
/// pipeline.register(Arc::new(WhitespaceProcessor::new()));
/// pipeline.register(Arc::new(CommentProcessor::new()));
/// pipeline.register(Arc::new(ArcExpander::new()));
///
/// let commands = pipeline.process_commands(&input_commands)?;
/// ```
pub struct ProcessorPipeline {
    processors: Vec<ProcessorHandle>,
    config: ProcessorConfig,
}

impl ProcessorPipeline {
    /// Create a new empty processor pipeline
    pub fn new() -> Self {
        Self {
            processors: Vec::new(),
            config: ProcessorConfig::new(),
        }
    }

    /// Register a processor in the pipeline
    ///
    /// Processors are applied in the order they are registered.
    pub fn register(&mut self, processor: ProcessorHandle) -> &mut Self {
        self.processors.push(processor);
        self
    }

    /// Register multiple processors at once
    pub fn register_all(&mut self, processors: Vec<ProcessorHandle>) -> &mut Self {
        self.processors.extend(processors);
        self
    }

    /// Get the number of registered processors
    pub fn processor_count(&self) -> usize {
        self.processors.len()
    }

    /// Get a reference to a processor by index
    pub fn get_processor(&self, index: usize) -> Option<&ProcessorHandle> {
        self.processors.get(index)
    }

    /// Get a reference to a processor by name
    pub fn get_processor_by_name(&self, name: &str) -> Option<&ProcessorHandle> {
        self.processors.iter().find(|p| p.name() == name)
    }

    /// List all registered processors
    pub fn list_processors(&self) -> Vec<(&str, &str, bool)> {
        self.processors
            .iter()
            .map(|p| (p.name(), p.description(), p.is_enabled()))
            .collect()
    }

    /// Process a single command through the entire pipeline
    ///
    /// Returns a vector of commands. Most processors return one command,
    /// but some may expand or skip commands.
    pub fn process_command(
        &self,
        command: &GcodeCommand,
        state: &GcodeState,
    ) -> Result<Vec<GcodeCommand>, String> {
        let mut current_commands = vec![command.clone()];

        for processor in &self.processors {
            if !processor.is_enabled() {
                continue;
            }

            let mut next_commands = Vec::new();

            for cmd in current_commands {
                match processor.process(&cmd, state) {
                    Ok(processed) => {
                        next_commands.extend(processed);
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Processor '{}' failed on command '{}': {}",
                            processor.name(),
                            cmd.command,
                            e
                        );
                        return Err(format!("Processor '{}' error: {}", processor.name(), e));
                    }
                }
            }

            current_commands = next_commands;

            // If no commands remain after processing, we can stop early
            if current_commands.is_empty() {
                break;
            }
        }

        Ok(current_commands)
    }

    /// Process a batch of commands through the pipeline
    ///
    /// # Arguments
    /// * `commands` - The commands to process
    /// * `state` - Current G-Code state (will be updated as commands are processed)
    ///
    /// # Returns
    /// A vector of processed commands
    pub fn process_commands(
        &self,
        commands: &[GcodeCommand],
        state: &mut GcodeState,
    ) -> Result<Vec<GcodeCommand>, String> {
        let mut results = Vec::new();

        for command in commands {
            let processed = self.process_command(command, state)?;

            // Update state based on processed commands
            for cmd in &processed {
                self.update_state(cmd, state)?;
                results.push(cmd.clone());
            }
        }

        Ok(results)
    }

    /// Update G-Code state based on a command
    fn update_state(&self, command: &GcodeCommand, state: &mut GcodeState) -> Result<(), String> {
        let cmd_upper = command.command.to_uppercase();

        // Motion mode
        if cmd_upper.contains("G00") {
            state.set_motion_mode(0)?;
        } else if cmd_upper.contains("G01") {
            state.set_motion_mode(1)?;
        } else if cmd_upper.contains("G02") {
            state.set_motion_mode(2)?;
        } else if cmd_upper.contains("G03") {
            state.set_motion_mode(3)?;
        }

        // Plane selection
        if cmd_upper.contains("G17") {
            state.set_plane_mode(17)?;
        } else if cmd_upper.contains("G18") {
            state.set_plane_mode(18)?;
        } else if cmd_upper.contains("G19") {
            state.set_plane_mode(19)?;
        }

        // Distance mode
        if cmd_upper.contains("G90") {
            state.set_distance_mode(90)?;
        } else if cmd_upper.contains("G91") {
            state.set_distance_mode(91)?;
        }

        // Feed rate mode
        if cmd_upper.contains("G93") {
            state.set_feed_rate_mode(93)?;
        } else if cmd_upper.contains("G94") {
            state.set_feed_rate_mode(94)?;
        } else if cmd_upper.contains("G95") {
            state.set_feed_rate_mode(95)?;
        }

        // Units
        if cmd_upper.contains("G20") {
            state.set_units_mode(20)?;
        } else if cmd_upper.contains("G21") {
            state.set_units_mode(21)?;
        }

        // Coordinate system (G54-G59)
        for cs in 54..=59 {
            if cmd_upper.contains(&format!("G{}", cs)) {
                state.set_coordinate_system(cs as u8)?;
                break;
            }
        }

        Ok(())
    }

    /// Clear all processors from the pipeline
    pub fn clear(&mut self) {
        self.processors.clear();
    }

    /// Get mutable access to the pipeline configuration
    pub fn config_mut(&mut self) -> &mut ProcessorConfig {
        &mut self.config
    }

    /// Get the pipeline configuration
    pub fn config(&self) -> &ProcessorConfig {
        &self.config
    }
}

impl Default for ProcessorPipeline {
    fn default() -> Self {
        Self::new()
    }
}

/// Processor registry for managing available processors
///
/// Maintains a registry of all available command processors and provides
/// factory methods for creating processor pipelines.
pub struct ProcessorRegistry {
    factories: std::collections::HashMap<String, Arc<dyn Fn() -> ProcessorHandle>>,
}

impl ProcessorRegistry {
    /// Create a new processor registry
    pub fn new() -> Self {
        Self {
            factories: std::collections::HashMap::new(),
        }
    }

    /// Register a processor factory
    pub fn register<F>(&mut self, name: impl Into<String>, factory: F) -> &mut Self
    where
        F: Fn() -> ProcessorHandle + Send + Sync + 'static,
    {
        self.factories.insert(name.into(), Arc::new(factory));
        self
    }

    /// Create a processor by name
    pub fn create(&self, name: &str) -> Option<ProcessorHandle> {
        self.factories.get(name).map(|f| f())
    }

    /// Create a pipeline with the specified processor names
    pub fn create_pipeline(&self, names: &[&str]) -> Result<ProcessorPipeline, String> {
        let mut pipeline = ProcessorPipeline::new();

        for name in names {
            match self.create(name) {
                Some(processor) => {
                    pipeline.register(processor);
                }
                None => {
                    return Err(format!("Unknown processor: {}", name));
                }
            }
        }

        Ok(pipeline)
    }

    /// List all registered processor names
    pub fn list_registered(&self) -> Vec<&str> {
        self.factories.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for ProcessorRegistry {
    fn default() -> Self {
        Self::new()
    }
}
