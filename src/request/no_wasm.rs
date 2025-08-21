//! No-WASM HTTP client implementation using reqwest
//! 
//! This module provides HTTP functionality for non-WASM environments
//! using the reqwest crate for making HTTP requests.

use crate::error::{ErrorKind, Result};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
};
use serde_json::Value;
use std::collections::HashMap;

use super::{CourseQueryParams, CourseSelectParams, HttpClient, LoginParams, RequestApi};

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
        let resp = self.client.get(index_url).send().await?;
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
            .ok_or_else(|| {
                ErrorKind::ParseError("Failed to extract AES key".to_string())
            })?;

        Ok(key)
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

    async fn send_login_request(&self, params: LoginParams<'_>) -> Result<Value> {
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
            HeaderValue::from_str(token)
                .map_err(|e| ErrorKind::ParseError(e.to_string()))?,
        );

        let resp = self
            .client
            .post(url)
            .headers(headers)
            .query(&params)
            .send()
            .await?;

        Ok(resp.json::<Value>().await?)
    }

    async fn get_selected_courses(&self, params: CourseQueryParams<'_>) -> Result<Value> {
        let url = "https://icourses.jlu.edu.cn/xsxk/elective/select";
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_str(params.token)
                .map_err(|e| ErrorKind::ParseError(e.to_string()))?,
        );
        headers.insert(
            "batchId",
            HeaderValue::from_str(params.batch_id)
                .map_err(|e| ErrorKind::ParseError(e.to_string()))?,
        );

        let resp = self.client.post(url).headers(headers).send().await?;

        Ok(resp.json::<Value>().await?)
    }

    async fn get_favorite_courses(&self, params: CourseQueryParams<'_>) -> Result<Value> {
        let url = "https://icourses.jlu.edu.cn/xsxk/sc/clazz/list";
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_str(params.token)
                .map_err(|e| ErrorKind::ParseError(e.to_string()))?,
        );
        headers.insert(
            "batchId",
            HeaderValue::from_str(params.batch_id)
                .map_err(|e| ErrorKind::ParseError(e.to_string()))?,
        );

        let resp = self.client.post(url).headers(headers).send().await?;

        Ok(resp.json::<Value>().await?)
    }

    async fn select_course(&self, params: CourseSelectParams<'_>) -> Result<Value> {
        let url = "https://icourses.jlu.edu.cn/xsxk/sc/clazz/addxk";
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_str(params.token)
                .map_err(|e| ErrorKind::ParseError(e.to_string()))?,
        );
        headers.insert(
            "batchId",
            HeaderValue::from_str(params.batch_id)
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
    let wrapper = NoWasmClient { client: client.clone() };
    wrapper.get_aes_key().await
}

pub async fn get_captcha(client: &Client) -> Result<(String, String)> {
    let wrapper = NoWasmClient { client: client.clone() };
    wrapper.get_captcha().await
}

pub async fn send_login_request(
    client: &Client,
    username: &str,
    encrypted_password: &str,
    captcha: &str,
    uuid: &str,
) -> Result<Value> {
    let wrapper = NoWasmClient { client: client.clone() };
    let params = LoginParams {
        username,
        encrypted_password,
        captcha,
        uuid,
    };
    wrapper.send_login_request(params).await
}

pub async fn set_batch(client: &Client, batch_id: &str, token: &str) -> Result<Value> {
    let wrapper = NoWasmClient { client: client.clone() };
    wrapper.set_batch(batch_id, token).await
}

pub async fn get_selected_courses(client: &Client, token: &str, batch_id: &str) -> Result<Value> {
    let wrapper = NoWasmClient { client: client.clone() };
    let params = CourseQueryParams { token, batch_id };
    wrapper.get_selected_courses(params).await
}

pub async fn get_favorite_courses(client: &Client, token: &str, batch_id: &str) -> Result<Value> {
    let wrapper = NoWasmClient { client: client.clone() };
    let params = CourseQueryParams { token, batch_id };
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
    let wrapper = NoWasmClient { client: client.clone() };
    let params = CourseSelectParams {
        token,
        batch_id,
        class_type,
        class_id,
        secret_val,
    };
    wrapper.select_course(params).await
}
