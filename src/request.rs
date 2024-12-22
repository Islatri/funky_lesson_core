use reqwest::{Client, header::{HeaderMap, HeaderValue}};
use serde_json::Value;
use std::collections::HashMap;
use crate::error::Result;

pub async fn create_client() -> Result<Client> {
    Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(Into::into)
}

pub async fn get_aes_key(client: &Client) -> Result<Vec<u8>> {
    let index_url = "https://icourses.jlu.edu.cn/";
    let resp = client.get(index_url).send().await?;
    let html = resp.text().await?;
    
    // Extract AES key from HTML
    let key = html
        .find("loginVue.loginForm.aesKey")
        .and_then(|start| {
            html[start..].find('"').map(|key_start| {
                html[start + key_start + 1..]
                    .find('"')
                    .map(|key_end| {
                        html[start + key_start + 1..start + key_start + 1 + key_end]
                            .as_bytes()
                            .to_vec()
                    })
            })
        })
        .flatten()
        .ok_or_else(|| crate::error::ErrorKind::ParseError("Failed to extract AES key".to_string()))?;

    Ok(key)
}

pub async fn get_captcha(client: &Client) -> Result<(String, String)> {
    let captcha_url = "https://icourses.jlu.edu.cn/xsxk/auth/captcha";
    let resp = client.post(captcha_url).send().await?;
    let captcha_data = resp.json::<Value>().await?;
    
    let uuid = captcha_data["data"]["uuid"]
        .as_str()
        .ok_or_else(|| crate::error::ErrorKind::ParseError("Invalid captcha uuid".to_string()))?
        .to_string();
    
    let captcha = captcha_data["data"]["captcha"]
        .as_str()
        .ok_or_else(|| crate::error::ErrorKind::ParseError("Invalid captcha data".to_string()))?
        .to_string();

    Ok((uuid, captcha))
}

pub async fn send_login_request(
    client: &Client,
    username: &str,
    encrypted_password: &str,
    captcha: &str,
    uuid: &str
) -> Result<Value> {
    let login_url = "https://icourses.jlu.edu.cn/xsxk/auth/login";
    
    let mut params = HashMap::new();
    params.insert("loginname", username);
    params.insert("password", encrypted_password);
    params.insert("captcha", captcha);
    params.insert("uuid", uuid);

    let resp = client
        .post(login_url)
        .query(&params)
        .send()
        .await?;

    resp.json::<Value>().await.map_err(Into::into)
}

pub async fn set_batch(
    client: &Client,
    batch_id: &str,
    token: &str
) -> Result<Value> {
    let url = "https://icourses.jlu.edu.cn/xsxk/elective/user";
    let mut params = HashMap::new();
    params.insert("batchId", batch_id);

    let mut headers = HeaderMap::new();
    headers.insert(
        "Authorization", 
        HeaderValue::from_str(token).map_err(|e| crate::error::ErrorKind::ParseError(e.to_string()))?
    );

    let resp = client
        .post(url)
        .headers(headers)
        .query(&params)
        .send()
        .await?;

    resp.json::<Value>().await.map_err(Into::into)
}

pub async fn get_selected_courses(
    client: &Client,
    token: &str,
    batch_id: &str
) -> Result<Value> {
    let url = "https://icourses.jlu.edu.cn/xsxk/elective/select";
    let mut headers = HeaderMap::new();
    headers.insert(
        "Authorization",
        HeaderValue::from_str(token).map_err(|e| crate::error::ErrorKind::ParseError(e.to_string()))?
    );
    headers.insert(
        "batchId",
        HeaderValue::from_str(batch_id).map_err(|e| crate::error::ErrorKind::ParseError(e.to_string()))?
    );

    println!("headers: {:?}", headers);

    let resp = client
        .post(url)
        .headers(headers)
        .send()
        .await?;

    
    resp.json::<Value>().await.map_err(Into::into)
}

pub async fn get_favorite_courses(
    client: &Client,
    token: &str,
    batch_id: &str
) -> Result<Value> {
    let url = "https://icourses.jlu.edu.cn/xsxk/sc/clazz/list";
    let mut headers = HeaderMap::new();
    headers.insert(
        "Authorization",
        HeaderValue::from_str(token).map_err(|e| crate::error::ErrorKind::ParseError(e.to_string()))?
    );
    headers.insert(
        "batchId",
        HeaderValue::from_str(batch_id).map_err(|e| crate::error::ErrorKind::ParseError(e.to_string()))?
    );

    println!("headers: {:?}", headers);

    let resp = client
        .post(url)
        .headers(headers)
        .send()
        .await?;

    resp.json::<Value>().await.map_err(Into::into)
}

pub async fn select_course(
    client: &Client,
    token: &str,
    batch_id: &str,
    class_type: &str,
    class_id: &str,
    secret_val: &str
) -> Result<Value> {
    let url = "https://icourses.jlu.edu.cn/xsxk/sc/clazz/addxk";
    let mut headers = HeaderMap::new();
    headers.insert(
        "Authorization",
        HeaderValue::from_str(token).map_err(|e| crate::error::ErrorKind::ParseError(e.to_string()))?
    );
    headers.insert(
        "batchId",
        HeaderValue::from_str(batch_id).map_err(|e| crate::error::ErrorKind::ParseError(e.to_string()))?
    );

    let mut params = HashMap::new();
    params.insert("clazzType", class_type);
    params.insert("clazzId", class_id);
    params.insert("secretVal", secret_val);

    let resp = client
        .post(url)
        .headers(headers)
        .query(&params)
        .send()
        .await?;

    resp.json::<Value>().await.map_err(Into::into)
}

