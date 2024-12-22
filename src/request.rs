#[cfg(feature = "no-wasm")]
use reqwest::{Client, header::{HeaderMap, HeaderValue}};
use serde_json::Value;
use std::collections::HashMap;
use crate::error::Result;


#[cfg(feature = "no-wasm")]
pub async fn create_client() -> Result<Client> {
    Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(Into::into)
}


#[cfg(feature = "no-wasm")]
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

#[cfg(feature = "no-wasm")]
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

#[cfg(feature = "no-wasm")]
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

#[cfg(feature = "no-wasm")]
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

#[cfg(feature = "no-wasm")]
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

#[cfg(feature = "no-wasm")]
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

#[cfg(feature = "no-wasm")]
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


#[cfg(feature = "wasm")]
use gloo_net::http::{Request, RequestBuilder};
#[cfg(feature = "wasm")]
use web_sys::RequestInit;

// Helper function to convert JsValue errors to our error type
#[cfg(feature = "wasm")]
fn js_err_to_string(err: impl std::fmt::Debug) -> crate::error::ErrorKind {
    crate::error::ErrorKind::ParseError(format!("{:?}", err))
}

// 我们不需要创建客户端，因为 gloo_net 会使用浏览器的 fetch API
#[cfg(feature = "wasm")]
pub async fn create_client() -> Result<()> {
    Ok(())
}

#[cfg(feature = "wasm")]
pub async fn get_aes_key() -> Result<Vec<u8>> {
    let index_url = "https://icourses.jlu.edu.cn/";
    let resp = Request::get(index_url)
        .send()
        .await?;
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

#[cfg(feature = "wasm")]
pub async fn get_captcha() -> Result<(String, String)> {
    let captcha_url = "https://icourses.jlu.edu.cn/xsxk/auth/captcha";
    let resp = Request::post(captcha_url)
        .send()
        .await?;
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

#[cfg(feature = "wasm")]
pub async fn send_login_request(
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

    let resp = Request::post(login_url)
        .query(params)
        .send()
        .await?;

    resp.json::<Value>().await.map_err(Into::into)
}

#[cfg(feature = "wasm")]
pub async fn set_batch(
    batch_id: &str,
    token: &str
) -> Result<Value> {
    let url = "https://icourses.jlu.edu.cn/xsxk/elective/user";
    let mut params = HashMap::new();
    params.insert("batchId", batch_id);

    let resp = Request::post(url)
        .header("Authorization", token)
        .query(params)
        .send()
        .await?;

    resp.json::<Value>().await.map_err(Into::into)
}

#[cfg(feature = "wasm")]
pub async fn get_selected_courses(
    token: &str,
    batch_id: &str
) -> Result<Value> {
    let url = "https://icourses.jlu.edu.cn/xsxk/elective/select";
    
    let resp = Request::post(url)
        .header("Authorization", token)
        .header("batchId", batch_id)
        .send()
        .await?;

    resp.json::<Value>().await.map_err(Into::into)
}

#[cfg(feature = "wasm")]
pub async fn get_favorite_courses(
    token: &str,
    batch_id: &str
) -> Result<Value> {
    let url = "https://icourses.jlu.edu.cn/xsxk/sc/clazz/list";
    
    let resp = Request::post(url)
        .header("Authorization", token)
        .header("batchId", batch_id)
        .send()
        .await?;

    resp.json::<Value>().await.map_err(Into::into)
}

#[cfg(feature = "wasm")]
pub async fn select_course(
    token: &str,
    batch_id: &str,
    class_type: &str,
    class_id: &str,
    secret_val: &str
) -> Result<Value> {
    let url = "https://icourses.jlu.edu.cn/xsxk/sc/clazz/addxk";
    
    let mut params = HashMap::new();
    params.insert("clazzType", class_type);
    params.insert("clazzId", class_id);
    params.insert("secretVal", secret_val);

    let resp = Request::post(url)
        .header("Authorization", token)
        .header("batchId", batch_id)
        .query(params)
        .send()
        .await?;

    resp.json::<Value>().await.map_err(Into::into)
}

// 为支持跨域请求添加 CORS 预检请求处理
#[cfg(feature = "wasm")]
async fn handle_preflight(url: &str) -> Result<()> {
    let mut opts = RequestInit::new();
    opts.method("OPTIONS");

    let request = web_sys::Request::new_with_str_and_init(url, &opts)
        .map_err(|e| crate::error::ErrorKind::ParseError(format!("{:?}", e)))?;
    
    // 添加 CORS 相关头部
    request.headers().set("Access-Control-Request-Method", "POST")
        .map_err(|e| crate::error::ErrorKind::ParseError(format!("{:?}", e)))?;
    request.headers().set("Access-Control-Request-Headers", "authorization,batchid")
        .map_err(|e| crate::error::ErrorKind::ParseError(format!("{:?}", e)))?;
    
    let window = web_sys::window().unwrap();
    let resp_promise = window.fetch_with_request(&request);
    
    // 等待预检请求完成
    wasm_bindgen_futures::JsFuture::from(resp_promise).await
        .map_err(|e| crate::error::ErrorKind::ParseError(format!("{:?}", e)))?;
    
    Ok(())
}