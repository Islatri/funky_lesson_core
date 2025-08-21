//! Request module - handles HTTP requests for both WASM and no-WASM environments
//! 
//! This module provides a unified interface for making HTTP requests while
//! supporting different implementations for WASM (gloo_net) and no-WASM (reqwest) environments.

#![allow(async_fn_in_trait)] // 允许在内部 trait 中使用 async fn

use crate::error::Result;
use serde_json::Value;

#[cfg(feature = "no-wasm")]
mod no_wasm;
#[cfg(feature = "no-wasm")]
pub use no_wasm::*;

#[cfg(feature = "wasm")]
mod wasm;
#[cfg(feature = "wasm")]
pub use wasm::*;

/// Common trait for HTTP client functionality
pub trait HttpClient {
    /// Create a new HTTP client instance
    async fn new() -> Result<Self>
    where
        Self: Sized;
}

/// Common parameters for login requests
#[derive(Debug, Clone)]
pub struct LoginParams<'a> {
    pub username: &'a str,
    pub encrypted_password: &'a str,
    pub captcha: &'a str,
    pub uuid: &'a str,
}

/// Common parameters for course selection
#[derive(Debug, Clone)]
pub struct CourseSelectParams<'a> {
    pub token: &'a str,
    pub batch_id: &'a str,
    pub class_type: &'a str,
    pub class_id: &'a str,
    pub secret_val: &'a str,
}

/// Common parameters for course queries
#[derive(Debug, Clone)]
pub struct CourseQueryParams<'a> {
    pub token: &'a str,
    pub batch_id: &'a str,
}

/// Common interface for all HTTP operations
pub trait RequestApi {
    /// Get AES encryption key from the server
    async fn get_aes_key(&self) -> Result<Vec<u8>>;
    
    /// Get captcha image and UUID
    async fn get_captcha(&self) -> Result<(String, String)>;
    
    /// Send login request with credentials
    async fn send_login_request(&self, params: LoginParams<'_>) -> Result<Value>;
    
    /// Set the current batch for course selection
    async fn set_batch(&self, batch_id: &str, token: &str) -> Result<Value>;
    
    /// Get list of selected courses
    async fn get_selected_courses(&self, params: CourseQueryParams<'_>) -> Result<Value>;
    
    /// Get list of favorite courses
    async fn get_favorite_courses(&self, params: CourseQueryParams<'_>) -> Result<Value>;
    
    /// Select a course
    async fn select_course(&self, params: CourseSelectParams<'_>) -> Result<Value>;
}
