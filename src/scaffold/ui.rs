//! UI — scaffold for future TUI / web frontends.
//!
//! **Status:** No TUI or web server in the binary. Safe to ignore unless extending this API.

/// UI configuration
#[derive(Debug, Clone)]
pub struct UiConfig {
    /// UI type (tui, web, none)
    pub ui_type: String,

    /// Port for web UI
    pub web_port: Option<u16>,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            ui_type: "none".to_string(),
            web_port: None,
        }
    }
}
