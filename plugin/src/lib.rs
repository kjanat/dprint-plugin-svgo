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

pub use handler::*;
