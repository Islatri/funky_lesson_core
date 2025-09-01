//! Application module - handles core application logic
//! 
//! This module provides the main application functionality, including
//! login, course management, and enrollment logic, with platform-specific
//! implementations for WASM and no-WASM environments.

// Platform-specific modules
#[cfg(feature = "no-wasm")]
pub mod request;
#[cfg(feature = "no-wasm")]
pub use request::*;

#[cfg(feature = "wasm")]
pub mod gloo;
#[cfg(feature = "wasm")]
pub use gloo::*;

// Re-export common data structures
