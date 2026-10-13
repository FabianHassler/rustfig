//! Errors reported by commands and tools.

use std::fmt;

/// A command failure, reported to the user (or the remote caller).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineError {
    /// Machine-readable code: `unknown_command`, `disabled`, `bad_params`,
    /// `io`, `format`, `not_found`, `internal`.
    pub code: &'static str,
    pub message: String,
}

impl EngineError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        EngineError {
            code,
            message: message.into(),
        }
    }
    pub fn unknown_command(id: &str) -> Self {
        Self::new("unknown_command", format!("unknown command {id:?}"))
    }
    pub fn disabled(id: &str) -> Self {
        Self::new("disabled", format!("command {id:?} is not available right now"))
    }
    pub fn bad_params(m: impl Into<String>) -> Self {
        Self::new("bad_params", m)
    }
    pub fn io(m: impl Into<String>) -> Self {
        Self::new("io", m)
    }
    pub fn format(m: impl Into<String>) -> Self {
        Self::new("format", m)
    }
    pub fn not_found(m: impl Into<String>) -> Self {
        Self::new("not_found", m)
    }
    pub fn internal(m: impl Into<String>) -> Self {
        Self::new("internal", format!("internal error: {}", m.into()))
    }
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for EngineError {}
