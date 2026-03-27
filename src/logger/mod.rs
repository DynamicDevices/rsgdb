//! Logger module
//!
//! Enhanced logging with structured output and filtering.

/// Logger configuration
#[derive(Debug, Clone)]
pub struct LoggerConfig {
    /// Log level
    pub level: String,
    
    /// Output format (text, json)
    pub format: String,
    
    /// Output file path
    pub output: Option<String>,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "text".to_string(),
            output: None,
        }
    }
}

// Made with Bob
