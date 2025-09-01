//! Request module - handles HTTP requests for both WASM and no-WASM environments
//!
//! This module provides a unified interface for making HTTP requests while
//! supporting different implementations for WASM (gloo_net) and no-WASM (reqwest) environments.

#[cfg(feature = "no-wasm")]
pub mod request;

#[cfg(feature = "wasm")]
pub mod gloo;
