//! Hust Conductor: dispatches user commands to exactly one language core.

pub mod app;
pub mod cli;
pub mod config;
pub mod core;
pub mod error;
pub mod json;
pub mod target;
pub mod tempdir;

pub use error::Error;

/// Protocol version understood by this conductor.
pub const PROTOCOL: u32 = 1;
/// Conductor version reported by `hustc --version`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
