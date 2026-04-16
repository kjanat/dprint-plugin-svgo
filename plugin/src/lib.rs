//! # dprint-plugin-svgo
//!
//! A dprint plugin for formatting and optimizing SVG files using SVGO.
//!
//! This plugin wraps SVGO to provide SVG optimization as part of the dprint
//! formatting ecosystem, allowing you to format SVG files alongside other code.

#![warn(missing_docs)]
#![allow(clippy::module_name_repetitions)]

extern crate dprint_core;

/// Configuration types for the SVGO plugin.
pub mod config;
/// Error types for the SVGO plugin.
pub mod error;
mod formatter;
mod handler;
mod process_loop;
mod process_messages;

pub use handler::*;

/// Logs guarded debug output for plugin process diagnostics.
pub fn debug_log(message: &str) {
  if std::env::var_os("SVGO_PLUGIN_DEBUG").is_some() {
    eprintln!("[svgo-plugin] {message}");
  }
}

pub use process_loop::handle_process_stdio_messages_sync;
