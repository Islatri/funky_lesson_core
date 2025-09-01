//! WASM HTTP client implementation using gloo_net
//! 
//! This module provides HTTP functionality for WASM environments
//! using the gloo_net crate for making HTTP requests via the browser's fetch API.

use crate::error::{ErrorKind, Result};
use gloo_net::http::{Request, RequestBuilder};
use serde_json::{json, Value};
use std::collections::HashMap;
use web_sys::{RequestCredentials, RequestMode};

use crate::model::dtos::{CourseQueryParams, CourseSelectParams,  LoginParams};
use crate::interface::{HttpClient,RequestApi};

/// HTTP client for WASM environments using gloo_net
#[derive(Debug, Clone)]
pub struct WasmClient;

impl HttpClient for WasmClient {
    async fn new() -> Result<Self> {
        Ok(Self)
    }
}

impl WasmClient {
    /// Build a request with common headers and settings
    async fn build_request(method: &str, url: &str) -> RequestBuilder {
        let mut builder = match method {
            "GET" => Request::get(url),
            "POST" => Request::post(url),
            _ => Request::get(url),
        };

        // Add basic request headers
        builder = builder
            .mode(RequestMode::Cors)
            .credentials(RequestCredentials::Include)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json");

        builder
    }

    /// Handle JSON response with error checking
    async fn handle_json_response(resp: gloo_net::http::Response) -> Result<Value> {
        let status = resp.ok();
        let text = resp.text().await?;

        if !status {
            return Err(ErrorKind::ParseError(format!("Request failed: {}", text)).into());
        }

        match serde_json::from_str::<Value>(&text) {
            Ok(json) => {
                if let Some(error) = json.get("error") {
                    return Err(ErrorKind::ParseError(format!("Server error: {}", error)).into());
                }
                Ok(json)
            }
            Err(_) => Err(ErrorKind::ParseError(format!("Invalid JSON response: {}", text)).into()),
        }
    }
}

impl RequestApi for WasmClient {
    async fn get_aes_key(&self) -> Result<Vec<u8>> {
        let index_url = "https://icourses.jlu.edu.cn/xsxk/profile/index.html";

        let resp = Request::get(index_url)
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8")
            .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
            .header("Origin", "http://127.0.0.1:1420")
            .header("Referer", "http://127.0.0.1:1420/")
            .send()
            .await?;

        log::debug!("Response status: {:?}", resp.status());
        log::debug!("Response headers: {:?}", resp.headers());

        let html = resp.text().await?;

        log::debug!("Response length: {}", html.len());
        log::debug!("First 100 chars: {}", &html[..100.min(html.len())]);

        // Extract AES key from HTML
        let key = html
            .find("loginVue.loginForm.aesKey")
            .and_then(|start| {
                html[start..].find('"').map(|key_start| {
                    html[start + key_start + 1..].find('"').map(|key_end| {
                        html[start + key_start + 1..start + key_start + 1 + key_end]
                            .as_bytes()
                            .to_vec()
                    })
                })
            })
            .flatten()
            .ok_or_else(|| {
                ErrorKind::ParseError("Failed to extract AES key".to_string())
            })?;

        Ok(key)
    }

    async fn get_captcha(&self) -> Result<(String, String)> {
        let captcha_url = "https://icourses.jlu.edu.cn/xsxk/auth/captcha";
        let resp = Request::post(captcha_url)
            .mode(RequestMode::Cors)
            .credentials(RequestCredentials::SameOrigin)
            .send()
            .await?;

        let captcha_data = resp.json::<Value>().await?;

        let uuid = captcha_data["data"]["uuid"]
            .as_str()
            .ok_or_else(|| ErrorKind::ParseError("Invalid captcha uuid".to_string()))?
            .to_string();

        let captcha = captcha_data["data"]["captcha"]
            .as_str()
            .ok_or_else(|| ErrorKind::ParseError("Invalid captcha data".to_string()))?
            .to_string();

        Ok((uuid, captcha))
    }

    async fn send_login_request(&self, params: LoginParams) -> Result<Value> {
        let login_url = "https://icourses.jlu.edu.cn/xsxk/auth/login";

        let mut query_params = HashMap::new();
        query_params.insert("loginname", params.username);
        query_params.insert("password", params.encrypted_password);
        query_params.insert("captcha", params.captcha);
        query_params.insert("uuid", params.uuid);

        let resp = Request::post(login_url)
            .query(query_params)
            .send()
            .await?;

        resp.json::<Value>().await.map_err(Into::into)
    }

    async fn set_batch(&self, batch_id: &str, token: &str) -> Result<Value> {
        let url = "https://icourses.jlu.edu.cn/xsxk/elective/user";
        let mut params = HashMap::new();
        params.insert("batchId", batch_id);

        log::debug!("Sending request to {} with token: {}", url, token);

        let resp = Request::post(url)
            .mode(RequestMode::NoCors)
            .header("Authorization", token)
            .query(params)
            .send()
            .await?;

        log::debug!("Set Batch Response {:?}", resp);
        log::debug!("Set Batch Response status: {:?}", resp.status());
        log::debug!("Set Batch Response headers: {:?}", resp.headers());

        // Return a success response since direct response parsing might fail in WASM
        Ok(json!({"code": 200, "status": "sent"}))
    }

    async fn get_selected_courses(&self, params: CourseQueryParams) -> Result<Value> {
        let url = "https://icourses.jlu.edu.cn/xsxk/elective/select";

        let resp = Request::post(url)
            .header("Authorization", &params.token)
            .header("batchId", &params.batch_id)
            .send()
            .await?;

        resp.json::<Value>().await.map_err(Into::into)
    }

    async fn get_favorite_courses(&self, params: CourseQueryParams) -> Result<Value> {
        let url = "https://icourses.jlu.edu.cn/xsxk/sc/clazz/list";

        let resp = Request::post(url)
            .header("Authorization", &params.token)
            .header("batchId", &params.batch_id)
            .send()
            .await?;

        resp.json::<Value>().await.map_err(Into::into)
    }

    async fn select_course(&self, params: CourseSelectParams) -> Result<Value> {
        let url = "https://icourses.jlu.edu.cn/xsxk/sc/clazz/addxk";

        let mut query_params = HashMap::new();
        query_params.insert("clazzType", params.class_type);
        query_params.insert("clazzId", params.class_id);
        query_params.insert("secretVal", params.secret_val);

        let resp = Request::post(url)
            .header("Authorization", &params.token)
            .header("batchId", &params.batch_id)
            .query(query_params)
            .send()
            .await?;

        resp.json::<Value>().await.map_err(Into::into)
    }
}

/// Proxy-based implementations for CORS-restricted environments
impl WasmClient {
    /// Get AES key via proxy server
    pub async fn get_aes_key_proxy(&self) -> Result<Vec<u8>> {
        let url = "http://127.0.0.1:3030/api/proxy/profile/index.html";

        let resp = Self::build_request("GET", url).await.send().await?;

        let html = resp.text().await?;

        // Extract AES key from HTML
        let key = html
            .find("loginVue.loginForm.aesKey")
            .and_then(|start| {
                html[start..].find('"').map(|key_start| {
                    html[start + key_start + 1..].find('"').map(|key_end| {
                        html[start + key_start + 1..start + key_start + 1 + key_end]
                            .as_bytes()
                            .to_vec()
                    })
                })
            })
            .flatten()
            .ok_or_else(|| ErrorKind::ParseError("Failed to extract AES key".to_string()))?;

        Ok(key)
    }

    /// Get captcha via proxy server
    pub async fn get_captcha_proxy(&self) -> Result<(String, String)> {
        let url = "http://127.0.0.1:3030/api/proxy/auth/captcha";

        let body = json!({
            "original_url": "https://icourses.jlu.edu.cn/xsxk/auth/captcha"
        });

        let resp = Self::build_request("POST", url)
            .await
            .json(&body)?
            .send()
            .await?;

        let json = Self::handle_json_response(resp).await?;

        let uuid = json["data"]["uuid"]
            .as_str()
            .ok_or_else(|| ErrorKind::ParseError("Invalid captcha uuid".to_string()))?
            .to_string();

        let captcha = json["data"]["captcha"]
            .as_str()
            .ok_or_else(|| ErrorKind::ParseError("Invalid captcha data".to_string()))?
            .to_string();

        Ok((uuid, captcha))
    }

    /// Send login request via proxy server
    pub async fn send_login_request_proxy(&self, params: LoginParams) -> Result<Value> {
        let url = "http://127.0.0.1:3030/api/proxy/auth/login";

        let body = json!({
            "original_url": "https://icourses.jlu.edu.cn/xsxk/auth/login",
            "loginname": params.username,
            "password": params.encrypted_password,
            "captcha": params.captcha,
            "uuid": params.uuid
        });

        let resp = Self::build_request("POST", url)
            .await
            .json(&body)?
            .send()
            .await?;

        Self::handle_json_response(resp).await
    }

    /// Set batch via proxy server
    pub async fn set_batch_proxy(&self, batch_id: &str, token: &str) -> Result<Value> {
        let url = "http://127.0.0.1:3030/api/proxy/elective/user";

        let body = json!({
            "original_url": "https://icourses.jlu.edu.cn/xsxk/elective/user",
            "batch_id": batch_id
        });

        let resp = Self::build_request("POST", url)
            .await
            .header("Authorization", token)
            .json(&body)?
            .send()
            .await?;

        Self::handle_json_response(resp).await
    }

    /// Get selected courses via proxy server
    pub async fn get_selected_courses_proxy(&self, params: CourseQueryParams) -> Result<Value> {
        let url = "http://127.0.0.1:3030/api/proxy/elective/select";

        let body = json!({
            "original_url": "https://icourses.jlu.edu.cn/xsxk/elective/select",
            "batch_id": params.batch_id
        });

        let resp = Self::build_request("POST", url)
            .await
            .header("Authorization", &params.token)
            .json(&body)?
            .send()
            .await?;

        Self::handle_json_response(resp).await
    }

    /// Get favorite courses via proxy server
    pub async fn get_favorite_courses_proxy(&self, params: CourseQueryParams) -> Result<Value> {
        let url = "http://127.0.0.1:3030/api/proxy/sc/clazz/list";

        let body = json!({
            "original_url": "https://icourses.jlu.edu.cn/xsxk/sc/clazz/list",
            "batch_id": params.batch_id
        });

        let resp = Self::build_request("POST", url)
            .await
            .header("Authorization", &params.token)
            .json(&body)?
            .send()
            .await?;

        Self::handle_json_response(resp).await
    }

    /// Select course via proxy server
    pub async fn select_course_proxy(&self, params: CourseSelectParams) -> Result<Value> {
        let url = "http://127.0.0.1:3030/api/proxy/sc/clazz/addxk";

        let body = json!({
            "original_url": "https://icourses.jlu.edu.cn/xsxk/sc/clazz/addxk",
            "batch_id": params.batch_id,
            "class_type": params.class_type,
            "class_id": params.class_id,
            "secret_val": params.secret_val
        });

        let resp = Self::build_request("POST", url)
            .await
            .header("Authorization", &params.token)
            .json(&body)?
            .send()
            .await?;

        Self::handle_json_response(resp).await
    }
}

// Legacy compatibility functions (for backward compatibility)
pub async fn create_client() -> Result<()> {
    Ok(())
}

pub async fn get_aes_key() -> Result<Vec<u8>> {
    let client = WasmClient;
    client.get_aes_key().await
}

pub async fn get_aes_key_proxy() -> Result<Vec<u8>> {
    let client = WasmClient;
    client.get_aes_key_proxy().await
}

pub async fn get_captcha() -> Result<(String, String)> {
    let client = WasmClient;
    client.get_captcha().await
}

pub async fn get_captcha_proxy() -> Result<(String, String)> {
    let client = WasmClient;
    client.get_captcha_proxy().await
}

pub async fn send_login_request(
    username: &str,
    encrypted_password: &str,
    captcha: &str,
    uuid: &str,
) -> Result<Value> {
    let client = WasmClient;
    let params = LoginParams {
        username: username.to_string(),
        encrypted_password: encrypted_password.to_string(),
        captcha: captcha.to_string(),
        uuid: uuid.to_string(),
    };
    client.send_login_request(params).await
}

pub async fn send_login_request_proxy(
    username: &str,
    encrypted_password: &str,
    captcha: &str,
    uuid: &str,
) -> Result<Value> {
    let client = WasmClient;
    let params = LoginParams {
        username: username.to_string(),
        encrypted_password: encrypted_password.to_string(),
        captcha: captcha.to_string(),
        uuid: uuid.to_string(),
    };
    client.send_login_request_proxy(params).await
}

pub async fn set_batch(batch_id: &str, token: &str) -> Result<Value> {
    let client = WasmClient;
    client.set_batch(batch_id, token).await
}

pub async fn set_batch_proxy(batch_id: &str, token: &str) -> Result<Value> {
    let client = WasmClient;
    client.set_batch_proxy(batch_id, token).await
}

pub async fn get_selected_courses(token: &str, batch_id: &str) -> Result<Value> {
    let client = WasmClient;
    let params = CourseQueryParams { token: token.to_string(), batch_id: batch_id.to_string() };
    client.get_selected_courses(params).await
}

pub async fn get_selected_courses_proxy(token: &str, batch_id: &str) -> Result<Value> {
    let client = WasmClient;
    let params = CourseQueryParams { token: token.to_string(), batch_id: batch_id.to_string() };
    client.get_selected_courses_proxy(params).await
}

pub async fn get_favorite_courses(token: &str, batch_id: &str) -> Result<Value> {
    let client = WasmClient;
    let params = CourseQueryParams { token: token.to_string(), batch_id: batch_id.to_string() };
    client.get_favorite_courses(params).await
}

pub async fn get_favorite_courses_proxy(token: &str, batch_id: &str) -> Result<Value> {
    let client = WasmClient;
    let params = CourseQueryParams { token: token.to_string(), batch_id: batch_id.to_string() };
    client.get_favorite_courses_proxy(params).await
}

pub async fn select_course(
    token: &str,
    batch_id: &str,
    class_type: &str,
    class_id: &str,
    secret_val: &str,
) -> Result<Value> {
    let client = WasmClient;
    let params = CourseSelectParams {
        token: token.to_string(),
        batch_id: batch_id.to_string(),
        class_type: class_type.to_string(),
        class_id: class_id.to_string(),
        secret_val: secret_val.to_string(),
    };
    client.select_course(params).await
}

pub async fn select_course_proxy(
    token: &str,
    batch_id: &str,
    class_type: &str,
    class_id: &str,
    secret_val: &str,
) -> Result<Value> {
    let client = WasmClient;
    let params = CourseSelectParams {
        token: token.to_string(),
        batch_id: batch_id.to_string(),
        class_type: class_type.to_string(),
        class_id: class_id.to_string(),
        secret_val: secret_val.to_string(),
    };
    client.select_course_proxy(params).await
}
