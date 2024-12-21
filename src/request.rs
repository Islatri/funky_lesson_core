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


// #![allow(non_snake_case)]

// use reqwest::{Client, header::{HeaderMap, HeaderValue}};
// use serde::{Deserialize, Serialize};
// use std::collections::HashMap;
// use crate::error::{ErrorKind, Result};

// const BASE_URL: &str = "https://icourses.jlu.edu.cn";

// #[derive(Debug, Clone)]
// pub struct RequestClient {
//     client: Client,
//     token: Option<String>,
//     batch_id: Option<String>,
// }

// impl RequestClient {
//     pub fn new() -> Result<Self> {
//         let client = Client::builder()
//             .danger_accept_invalid_certs(true)
//             .build()?;

//         Ok(Self {
//             client,
//             token: None,
//             batch_id: None,
//         })
//     }

//     pub fn with_auth(mut self, token: String, batch_id: Option<String>) -> Self {
//         self.token = Some(token);
//         self.batch_id = batch_id;
//         self
//     }

//     fn create_auth_headers(&self) -> HeaderMap {
//         let mut headers = HeaderMap::new();
//         if let Some(token) = &self.token {
//             headers.insert(
//                 "Authorization",
//                 HeaderValue::from_str(token).unwrap(),
//             );
//         }
//         if let Some(batch_id) = &self.batch_id {
//             headers.insert(
//                 "batchId",
//                 HeaderValue::from_str(batch_id).unwrap(),
//             );
//         }
//         headers
//     }

//     pub async fn get_index_page(&self) -> Result<String> {
//         Ok(self.client.get(format!("{}/", BASE_URL))
//             .send()
//             .await?
//             .text()
//             .await?)
//     }

//     pub async fn get_captcha(&self) -> Result<CaptchaResponse> {
//         let url = format!("{}/xsxk/auth/captcha", BASE_URL);
//         Ok(self.client.post(&url)
//             .send()
//             .await?
//             .json()
//             .await?)
//     }

//     pub async fn login(
//         &self,
//         username: &str,
//         encrypted_password: &str,
//         captcha: &str,
//         uuid: &str,
//     ) -> Result<LoginResponse> {
//         let url = format!("{}/xsxk/auth/login", BASE_URL);
//         let mut params = HashMap::new();
//         params.insert("loginname", username);
//         params.insert("password", encrypted_password);
//         params.insert("captcha", captcha);
//         params.insert("uuid", uuid);

//         Ok(self.client.post(&url)
//             .query(&params)
//             .send()
//             .await?
//             .json()
//             .await?)
//     }

//     pub async fn set_batch(&self, batch_id: &str) -> Result<serde_json::Value> {
//         let url = format!("{}/xsxk/elective/user", BASE_URL);
//         let mut params = HashMap::new();
//         params.insert("batchId", batch_id);

//         Ok(self.client.post(&url)
//             .headers(self.create_auth_headers())
//             .query(&params)
//             .send()
//             .await?
//             .json()
//             .await?)
//     }

//     pub async fn get_selected_courses(&self) -> Result<Vec<CourseInfo>> {
//         let url = format!("{}/xsxk/elective/select", BASE_URL);
//         let resp: serde_json::Value = self.client.post(&url)
//             .headers(self.create_auth_headers())
//             .send()
//             .await?
//             .json()
//             .await?;

//         if resp["code"] == 200 {
//             Ok(serde_json::from_value(resp["data"].clone())?)
//         } else {
//             Err(ErrorKind::CourseError(format!("Failed to get selected courses: {}", resp["msg"])).into())
//         }
//     }

//     pub async fn get_favorite_courses(&self) -> Result<Vec<CourseInfo>> {
//         let url = format!("{}/xsxk/sc/clazz/list", BASE_URL);
//         let resp: serde_json::Value = self.client.post(&url)
//             .headers(self.create_auth_headers())
//             .send()
//             .await?
//             .json()
//             .await?;

//         if resp["code"] == 200 {
//             Ok(serde_json::from_value(resp["data"].clone())?)
//         } else {
//             Err(ErrorKind::CourseError(format!("Failed to get favorite courses: {}", resp["msg"])).into())
//         }
//     }

//     pub async fn select_course(
//         &self,
//         class_type: &str,
//         class_id: &str,
//         secret_val: &str,
//     ) -> Result<serde_json::Value> {
//         let url = format!("{}/xsxk/sc/clazz/addxk", BASE_URL);
//         let mut params = HashMap::new();
//         params.insert("clazzType", class_type);
//         params.insert("clazzId", class_id);
//         params.insert("secretVal", secret_val);

//         Ok(self.client.post(&url)
//             .headers(self.create_auth_headers())
//             .query(&params)
//             .send()
//             .await?
//             .json()
//             .await?)
//     }
// }

// #[derive(Debug, Deserialize, Serialize)]
// pub struct LoginResponse {
//     pub code: i32,
//     pub msg: String,
//     #[serde(default)]
//     pub data: Option<LoginData>,
// }

// #[derive(Debug, Deserialize, Serialize)]
// pub struct LoginData {
//     pub token: String,
//     pub student: StudentInfo,
// }

// #[derive(Debug, Deserialize, Serialize)]
// pub struct StudentInfo {
//     pub XH: String,
//     pub XM: String,
//     pub ZYMC: String,
//     #[serde(rename = "electiveBatchList")]
//     pub elective_batch_list: Vec<BatchInfo>,
// }

// #[derive(Debug, Deserialize, Serialize, Clone)]
// pub struct BatchInfo {
//     pub code: String,
//     pub name: String,
//     #[serde(rename = "beginTime")]
//     pub begin_time: String,
//     #[serde(rename = "endTime")]
//     pub end_time: String,
// }

// #[derive(Debug, Deserialize, Serialize)]
// pub struct CaptchaResponse {
//     pub code: i32,
//     pub data: CaptchaData,
// }

// #[derive(Debug, Deserialize, Serialize)]
// pub struct CaptchaData {
//     pub uuid: String,
//     pub captcha: String,
// }

// #[derive(Debug, Clone, Deserialize, Serialize)]
// pub struct CourseInfo {
//     pub SKJS: String,
//     pub KCM: String,
//     pub JXBID: String,
//     #[serde(rename = "teachingClassType")]
//     pub teaching_class_type: String,
//     #[serde(default, rename = "secretVal")]
//     pub secret_val: Option<String>,
// }