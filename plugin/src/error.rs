//! Error types for the SVGO plugin.

use thiserror::Error;

/// Errors that can occur during SVGO plugin operations.
#[derive(Error, Debug)]
pub enum SvgoError {
  /// Invalid configuration structure.
  #[error("Invalid config: expected object for extension override '{0}'")]
  InvalidExtensionOverride(String),

  /// Invalid UTF-8 in input file.
  #[error("Invalid UTF-8 in input: {0}")]
  InvalidUtf8(#[from] std::string::FromUtf8Error),

  /// JSON serialization error.
  #[error("JSON serialization failed: {0}")]
  JsonSerialization(#[from] serde_json::Error),

  /// V8 runtime error during formatting.
  #[error("Formatting failed: {0}")]
  Runtime(#[from] deno_core::anyhow::Error),
}
