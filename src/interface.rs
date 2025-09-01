#![allow(async_fn_in_trait)] // 允许在内部 trait 中使用 async fn

use crate::error::Result;
use crate::model::dtos::{CourseQueryParams, CourseSelectParams, LoginParams};
use serde_json::Value;

/// Common trait for HTTP client functionality
pub trait HttpClient {
    /// Create a new HTTP client instance
    async fn new() -> Result<Self>
    where
        Self: Sized;
}

/// Common interface for all HTTP operations
pub trait RequestApi {
    /// Get AES encryption key from the server
    async fn get_aes_key(&self) -> Result<Vec<u8>>;
    
    /// Get captcha image and UUID
    async fn get_captcha(&self) -> Result<(String, String)>;
    
    /// Send login request with credentials
    async fn send_login_request(&self, params: LoginParams) -> Result<Value>;
    
    /// Set the current batch for course selection
    async fn set_batch(&self, batch_id: &str, token: &str) -> Result<Value>;
    
    /// Get list of selected courses
    async fn get_selected_courses(&self, params: CourseQueryParams) -> Result<Value>;
    
    /// Get list of favorite courses
    async fn get_favorite_courses(&self, params: CourseQueryParams) -> Result<Value>;
    
    /// Select a course
    async fn select_course(&self, params: CourseSelectParams) -> Result<Value>;
}
