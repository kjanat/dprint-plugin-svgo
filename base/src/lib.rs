//! Helper library for creating dprint plugins with deno_core.
//!
//! This crate provides utilities for embedding V8 via deno_core, managing
//! thread pools with memory-aware scaling, and handling V8 snapshots.

#[cfg(feature = "build")]
pub mod build;
pub mod channel;
pub mod runtime;
pub mod snapshot;
pub mod util;

// Re-export key types for ergonomic imports
pub use channel::{Channel, CreateChannelOptions, Formatter};
pub use runtime::{CreateRuntimeOptions, JsRuntime};
pub use snapshot::deserialize_snapshot;
pub use util::{
  create_tokio_runtime, set_v8_max_memory, system_available_memory, try_create_tokio_runtime,
};
