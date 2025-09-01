//! No-WASM HTTP client implementation using reqwest
//!
//! This module provides HTTP functionality for non-WASM environments
//! using the reqwest crate for making HTTP requests.

use crate::error::{ErrorKind, Result};
use reqwest::{
    Client,
    header::{HeaderMap, HeaderValue},
};
use serde_json::Value;
use std::collections::HashMap;

use crate::interface::{HttpClient, RequestApi};
use crate::model::dtos::{CourseQueryParams, CourseSelectParams, LoginParams};

/// HTTP client for no-WASM environments using reqwest
#[derive(Debug, Clone)]
pub struct NoWasmClient {
    client: Client,
}

impl HttpClient for NoWasmClient {
    async fn new() -> Result<Self> {
        let client = Client::builder()
            .danger_accept_invalid_certs(true)
            .build()?;

        Ok(Self { client })
    }
}

impl RequestApi for NoWasmClient {
    async fn get_aes_key(&self) -> Result<Vec<u8>> {
        let index_url = "https://icourses.jlu.edu.cn/";

        // 添加重试机制
        for attempt in 1..=3 {
            match self.client.get(index_url).send().await {
                Ok(resp) => {
                    let status = resp.status();
                    if !status.is_success() {
                        eprintln!(
                            "HTTP error {}: {}",
                            status.as_u16(),
                            status.canonical_reason().unwrap_or("Unknown")
                        );
                        if attempt == 3 {
                            return Err(
                                ErrorKind::ParseError(format!("HTTP error: {status}")).into()
                            );
                        }
                        continue;
                    }

                    let html = match resp.text().await {
                        Ok(html) => html,
                        Err(e) => {
                            eprintln!("Failed to read response text: {e}");
                            if attempt == 3 {
                                return Err(e.into());
                            }
                            continue;
                        }
                    };

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
                            ErrorKind::ParseError("Failed to extract AES key from HTML".to_string())
                        })?;

                    println!("AES key extracted successfully");
                    return Ok(key);
                }
                Err(e) => {
                    eprintln!("Network error (attempt {attempt}/3): {e}");
                    if attempt == 3 {
                        return Err(e.into());
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                }
            }
        }

        unreachable!()
    }

    async fn get_captcha(&self) -> Result<(String, String)> {
        let captcha_url = "https://icourses.jlu.edu.cn/xsxk/auth/captcha";
        let resp = self.client.post(captcha_url).send().await?;
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

        let resp = self
            .client
            .post(login_url)
            .query(&query_params)
            .send()
            .await?;

        Ok(resp.json::<Value>().await?)
    }

    async fn set_batch(&self, batch_id: &str, token: &str) -> Result<Value> {
        let url = "https://icourses.jlu.edu.cn/xsxk/elective/user";
        let mut params = HashMap::new();
        params.insert("batchId", batch_id);

        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_str(token).map_err(|e| ErrorKind::ParseError(e.to_string()))?,
        );

        let resp = self
            .client
            .post(url)
            .headers(headers)
            .query(&params)
            .send()
            .await?;

        let get_url =
            format!("https://icourses.jlu.edu.cn/xsxk/elective/grablessons?batchId={batch_id}");
        self.client
            .get(&get_url)
            .header("Authorization", token)
            .header("Connection", "keep-alive")
            .send()
            .await?;

        Ok(resp.json::<Value>().await?)
    }

    async fn get_selected_courses(&self, params: CourseQueryParams) -> Result<Value> {
        let url = "https://icourses.jlu.edu.cn/xsxk/elective/select";
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&params.token)
                .map_err(|e| ErrorKind::ParseError(e.to_string()))?,
        );
        headers.insert(
            "batchId",
            HeaderValue::from_str(&params.batch_id)
                .map_err(|e| ErrorKind::ParseError(e.to_string()))?,
        );

        let resp = self.client.post(url).headers(headers).send().await?;

        Ok(resp.json::<Value>().await?)
    }

    async fn get_favorite_courses(&self, params: CourseQueryParams) -> Result<Value> {
        let url = "https://icourses.jlu.edu.cn/xsxk/sc/clazz/list";
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&params.token)
                .map_err(|e| ErrorKind::ParseError(e.to_string()))?,
        );
        headers.insert(
            "BatchId",
            HeaderValue::from_str(&params.batch_id)
                .map_err(|e| ErrorKind::ParseError(e.to_string()))?,
        );
        headers.insert(
            "Referer",
            HeaderValue::from_str(&format!(
                "https://icourses.jlu.edu.cn/xsxk/elective/grablessons?batchId={}",
                params.batch_id
            ))
            .map_err(|e| ErrorKind::ParseError(e.to_string()))?,
        );

        let resp = self.client.post(url).headers(headers).send().await?;

        Ok(resp.json::<Value>().await?)
    }

    async fn select_course(&self, params: CourseSelectParams) -> Result<Value> {
        let url = "https://icourses.jlu.edu.cn/xsxk/sc/clazz/addxk";
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&params.token)
                .map_err(|e| ErrorKind::ParseError(e.to_string()))?,
        );
        headers.insert(
            "batchId",
            HeaderValue::from_str(&params.batch_id)
                .map_err(|e| ErrorKind::ParseError(e.to_string()))?,
        );

        let mut query_params = HashMap::new();
        query_params.insert("clazzType", params.class_type);
        query_params.insert("clazzId", params.class_id);
        query_params.insert("secretVal", params.secret_val);

        let resp = self
            .client
            .post(url)
            .headers(headers)
            .query(&query_params)
            .send()
            .await?;

        Ok(resp.json::<Value>().await?)
    }
}

// Legacy compatibility functions that use the Client directly (for backward compatibility)
pub async fn create_client() -> Result<Client> {
    Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.into())
}

pub async fn get_aes_key(client: &Client) -> Result<Vec<u8>> {
    let wrapper = NoWasmClient {
        client: client.clone(),
    };
    wrapper.get_aes_key().await
}

pub async fn get_captcha(client: &Client) -> Result<(String, String)> {
    let wrapper = NoWasmClient {
        client: client.clone(),
    };
    wrapper.get_captcha().await
}

pub async fn send_login_request(
    client: &Client,
    username: &str,
    encrypted_password: &str,
    captcha: &str,
    uuid: &str,
) -> Result<Value> {
    let wrapper = NoWasmClient {
        client: client.clone(),
    };
    let params = LoginParams {
        username: username.to_string(),
        encrypted_password: encrypted_password.to_string(),
        captcha: captcha.to_string(),
        uuid: uuid.to_string(),
    };
    wrapper.send_login_request(params).await
}

pub async fn set_batch(client: &Client, batch_id: &str, token: &str) -> Result<Value> {
    let wrapper = NoWasmClient {
        client: client.clone(),
    };
    wrapper.set_batch(batch_id, token).await
}

pub async fn get_selected_courses(client: &Client, token: &str, batch_id: &str) -> Result<Value> {
    let wrapper = NoWasmClient {
        client: client.clone(),
    };
    let params = CourseQueryParams {
        token: token.to_string(),
        batch_id: batch_id.to_string(),
    };
    wrapper.get_selected_courses(params).await
}

pub async fn get_favorite_courses(client: &Client, token: &str, batch_id: &str) -> Result<Value> {
    let wrapper = NoWasmClient {
        client: client.clone(),
    };
    let params = CourseQueryParams {
        token: token.to_string(),
        batch_id: batch_id.to_string(),
    };
    wrapper.get_favorite_courses(params).await
}

pub async fn select_course(
    client: &Client,
    token: &str,
    batch_id: &str,
    class_type: &str,
    class_id: &str,
    secret_val: &str,
) -> Result<Value> {
    let wrapper = NoWasmClient {
        client: client.clone(),
    };
    let params = CourseSelectParams {
        token: token.to_string(),
        batch_id: batch_id.to_string(),
        class_type: class_type.to_string(),
        class_id: class_id.to_string(),
        secret_val: secret_val.to_string(),
    };
    wrapper.select_course(params).await
}
