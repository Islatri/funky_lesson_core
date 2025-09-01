#![allow(non_snake_case)]

use aes::Aes128;
use aes::cipher::{BlockEncryptMut, KeyInit, block_padding::Pkcs7, generic_array::GenericArray};
use futures::future::join_all;
use futures_util::{SinkExt, StreamExt};
use reqwest::{
    Client,
    header::{HeaderMap, HeaderValue},
};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{
    ClientConfig, DigitallySignedStruct, Error as TlsError, RootCertStore, SignatureScheme,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, Once},
    time::Duration,
};
use tokio_tungstenite::{Connector, connect_async_tls_with_config, tungstenite::protocol::Message};

type Aes128EcbEnc = ecb::Encryptor<Aes128>;
const WORK_THREAD_COUNT: usize = 8;

// 确保只初始化一次加密提供者
static INIT: Once = Once::new();

fn init_crypto_provider() {
    INIT.call_once(|| {
        rustls::crypto::aws_lc_rs::default_provider()
            .install_default()
            .expect("Failed to install default crypto provider");
    });
}

// 不验证证书的验证器
#[derive(Debug)]
struct NoVerification;

impl ServerCertVerifier for NoVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, TlsError> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, TlsError> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, TlsError> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PKCS1_SHA1,
            SignatureScheme::ECDSA_SHA1_Legacy,
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::ECDSA_NISTP521_SHA512,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::ED25519,
            SignatureScheme::ED448,
        ]
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct LoginResponse {
    code: i32,
    msg: String,
    #[serde(default)]
    data: Option<LoginData>,
}

#[derive(Debug, Deserialize, Serialize)]
struct LoginData {
    token: String,
    student: StudentInfo,
}

#[derive(Debug, Deserialize, Serialize)]
struct StudentInfo {
    XH: String,
    XM: String,
    ZYMC: String,
    #[serde(rename = "electiveBatchList")]
    elective_batch_list: Vec<BatchInfo>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct BatchInfo {
    code: String,
    name: String,
    #[serde(rename = "beginTime")]
    begin_time: String,
    #[serde(rename = "endTime")]
    end_time: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct CaptchaResponse {
    code: i32,
    data: CaptchaData,
}

#[derive(Debug, Deserialize, Serialize)]
struct CaptchaData {
    uuid: String,
    captcha: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct CourseInfo {
    SKJS: String,  // 教师名
    KCM: String,   // 课程名
    JXBID: String, // 教学班ID
    #[serde(rename = "teachingClassType")]
    teaching_class_type: String,
    #[serde(default, rename = "secretVal")]
    secret_val: Option<String>,
}

#[derive(Debug, Clone)]
struct ICourses {
    client: Client,
    aes_key: Vec<u8>,
    login_name: String,
    password: String,
    token: String,
    batch_id: String,
    batch_list: Vec<BatchInfo>,
    try_if_capacity_full: bool,
    selected_courses: Vec<CourseInfo>,
    favorite_courses: Vec<CourseInfo>,
}

impl ICourses {
    async fn new(username: String, password: String) -> Result<Self, Box<dyn std::error::Error>> {
        // 初始化加密提供者
        init_crypto_provider();

        let client = Client::builder()
            .danger_accept_invalid_certs(true)
            .build()?;

        Ok(ICourses {
            client,
            aes_key: Vec::new(),
            login_name: username,
            password,
            token: String::new(),
            batch_id: String::new(),
            batch_list: Vec::new(),
            try_if_capacity_full: true,
            selected_courses: Vec::new(),
            favorite_courses: Vec::new(),
        })
    }

    // WebSocket心跳维护函数
    async fn maintain_websocket_heartbeat(&self) -> Result<(), Box<dyn std::error::Error>> {
        let ws_url = format!(
            "wss://icourses.jlu.edu.cn/xsxk/websocket/{}",
            self.login_name
        );

        println!("正在连接WebSocket: {}", ws_url);

        // 创建一个不验证证书的TLS配置
        let config = ClientConfig::builder()
            .with_root_certificates(RootCertStore::empty())
            .with_no_client_auth();

        // 跳过证书验证
        let mut config = config;
        config
            .dangerous()
            .set_certificate_verifier(Arc::new(NoVerification));

        let connector = Connector::Rustls(Arc::new(config));

        let request = tokio_tungstenite::tungstenite::handshake::client::Request::builder()
            .uri(&ws_url)
            .header("Origin", "https://icourses.jlu.edu.cn")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/139.0.0.0 Safari/537.36")
            .header("Cookie", format!("Authorization={}", self.token))
            .body(())?;

        match connect_async_tls_with_config(request, None, false, Some(connector)).await {
            Ok((ws_stream, _)) => {
                println!("✅ WebSocket连接成功！");
                let (mut write, mut read) = ws_stream.split();

                // 启动心跳发送任务
                let heartbeat_task = tokio::spawn(async move {
                    let mut interval = tokio::time::interval(Duration::from_secs(5)); // 每5秒发送一次心跳
                    loop {
                        interval.tick().await;
                        if let Err(e) = write.send(Message::Text("hi".to_string().into())).await {
                            println!("❌ WebSocket心跳发送失败: {}", e);
                            break;
                        }
                        println!("💓 WebSocket心跳已发送: hi");
                    }
                });

                // 启动消息接收任务
                let receive_task = tokio::spawn(async move {
                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                println!("📨 WebSocket收到响应: {}", text);
                            }
                            Ok(Message::Close(_)) => {
                                println!("🔒 WebSocket连接已关闭");
                                break;
                            }
                            Err(e) => {
                                println!("❌ WebSocket接收消息错误: {}", e);
                                break;
                            }
                            _ => {}
                        }
                    }
                });

                // 等待任一任务完成
                tokio::select! {
                    _ = heartbeat_task => {
                        println!("💔 心跳任务结束");
                    }
                    _ = receive_task => {
                        println!("📭 接收任务结束");
                    }
                }
            }
            Err(e) => {
                println!("❌ WebSocket连接失败: {}", e);
                return Err(e.into());
            }
        }

        Ok(())
    }

    async fn login(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        // Get AES key from index page
        let index_url = "https://icourses.jlu.edu.cn/";
        let resp = self.client.get(index_url).send().await?;
        let html = resp.text().await?;

        // Extract AES key from HTML
        if let Some(start) = html.find("loginVue.loginForm.aesKey") {
            if let Some(key_start) = html[start..].find('"') {
                if let Some(key_end) = html[start + key_start + 1..].find('"') {
                    self.aes_key = html[start + key_start + 1..start + key_start + 1 + key_end]
                        .as_bytes()
                        .to_vec();
                }
            }
        }

        // Get captcha
        let captcha_url = "https://icourses.jlu.edu.cn/xsxk/auth/captcha";
        let resp = self.client.post(captcha_url).send().await?;
        let captcha_data: CaptchaResponse = resp.json().await?;

        let base64 = base64_simd::STANDARD;

        // Save captcha image to file
        let captcha_img =
            base64.decode_to_vec(captcha_data.data.captcha.split(',').nth(1).unwrap())?;
        fs::write("captcha.png", captcha_img)?;

        println!("Please check captcha.png and enter the captcha:");
        io::stdout().flush()?;

        let mut captcha = String::new();
        io::stdin().read_line(&mut captcha)?;
        let captcha = captcha.trim().to_string();

        // Encrypt password
        let srcs = self.password.as_bytes();
        let key = GenericArray::from_slice(&self.aes_key);
        let mut buf = [0u8; 128];
        let pt_len = srcs.len();
        buf[..pt_len].copy_from_slice(&srcs);
        let ct = Aes128EcbEnc::new(key.into())
            .encrypt_padded_mut::<Pkcs7>(&mut buf, pt_len)
            .unwrap();

        let password_b64 = base64.encode_to_string(ct);

        // Login request
        let login_url = "https://icourses.jlu.edu.cn/xsxk/auth/login";

        let username = self.login_name.clone();

        let mut params = HashMap::new();
        params.insert("loginname", &username);
        params.insert("password", &password_b64);
        params.insert("captcha", &captcha);
        params.insert("uuid", &captcha_data.data.uuid);

        let resp = self.client.post(login_url).query(&params).send().await?;

        let login_resp: LoginResponse = resp.json().await?;

        if login_resp.code == 200 && login_resp.msg == "登录成功" {
            if let Some(data) = login_resp.data {
                self.token = data.token;
                self.batch_list = data.student.elective_batch_list;

                println!("Login success!");
                println!("=====================================");
                println!("XH: {}", data.student.XH);
                println!("XM: {}", data.student.XM);
                println!("ZYMC: {}", data.student.ZYMC);
                println!("学号(login_name): {}", self.login_name);
                println!("=====================================");

                for batch in &self.batch_list {
                    println!("name: {}", batch.name);
                    println!("BeginTime: {}", batch.begin_time);
                    println!("EndTime: {}", batch.end_time);
                    println!("=====================================");
                }

                return Ok(true);
            }
        }

        println!("Login failed: {}", login_resp.msg);
        Ok(false)
    }

    async fn set_batch_id(&mut self, idx: usize) -> Result<(), Box<dyn std::error::Error>> {
        if idx >= self.batch_list.len() {
            println!("No such batch Id");
            return Ok(());
        }

        self.batch_id = self.batch_list[idx].code.clone();

        let url = "https://icourses.jlu.edu.cn/xsxk/elective/user";
        let mut params = HashMap::new();
        params.insert("batchId", &self.batch_id);

        let mut headers = HeaderMap::new();
        headers.insert("Authorization", HeaderValue::from_str(&self.token)?);

        let resp = self
            .client
            .post(url)
            .headers(headers)
            .query(&params)
            .send()
            .await?;

        let resp: serde_json::Value = resp.json().await?;

        if resp["code"] != 200 {
            println!("Set batchId failed");
            return Ok(());
        }

        let batch = &self.batch_list[idx];
        println!("Selected BatchId:");
        println!("=====================================");
        println!("name: {}", batch.name);
        println!("BeginTime: {}", batch.begin_time);
        println!("EndTime: {}", batch.end_time);
        println!("=====================================");

        Ok(())
    }

    // 获取已选课程列表
    async fn get_select(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let url = "https://icourses.jlu.edu.cn/xsxk/elective/select";
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", HeaderValue::from_str(&self.token)?);
        headers.insert("batchId", HeaderValue::from_str(&self.batch_id)?);

        let resp = self.client.post(url).headers(headers).send().await?;

        let resp_json: serde_json::Value = resp.json().await?;

        if resp_json["code"] == 200 {
            self.selected_courses = serde_json::from_value(resp_json["data"].clone())?;
            println!(
                "✅ 成功获取已选课程列表，共 {} 门课程",
                self.selected_courses.len()
            );
            Ok(())
        } else {
            println!("❌ 获取已选课程失败: {}", resp_json["msg"]);
            Err("获取已选课程失败".into())
        }
    }

    // 获取收藏课程列表
    async fn get_favorite(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let url = "https://icourses.jlu.edu.cn/xsxk/sc/clazz/list";
        let mut headers = HeaderMap::new();
        println!("Authorization: {}", &self.token);
        headers.insert("Authorization", HeaderValue::from_str(&self.token)?);
        headers.insert("batchId", HeaderValue::from_str(&self.batch_id)?);

        let resp = self.client.post(url).headers(headers).send().await?;

        let resp_json: serde_json::Value = resp.json().await?;

        if resp_json["code"] == 200 {
            self.favorite_courses = serde_json::from_value(resp_json["data"].clone())?;
            println!(
                "✅ 成功获取收藏课程列表，共 {} 门课程",
                self.favorite_courses.len()
            );
            Ok(())
        } else {
            println!("❌ 获取收藏课程失败: {}", resp_json["msg"]);
            Err("获取收藏课程失败".into())
        }
    }

    // 选择单个收藏课程
    #[allow(dead_code)]
    async fn select_favorite(
        &self,
        class_type: &str,
        class_id: &str,
        secret_val: &str,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let url = "https://icourses.jlu.edu.cn/xsxk/sc/clazz/addxk";
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", HeaderValue::from_str(&self.token)?);
        headers.insert("batchId", HeaderValue::from_str(&self.batch_id)?);

        let mut params = HashMap::new();
        params.insert("clazzType", class_type);
        params.insert("clazzId", class_id);
        params.insert("secretVal", secret_val);

        let resp = self
            .client
            .post(url)
            .headers(headers)
            .query(&params)
            .send()
            .await?;

        Ok(resp.json().await?)
    }

    // 打印已选课程
    fn print_select(&self) {
        println!("==================已选课程==================");
        for course in &self.selected_courses {
            println!(
                "教师: {:<10}课程: {:<20}ID: {:<30}",
                course.SKJS, course.KCM, course.JXBID
            );
        }
    }

    // 打印收藏课程
    fn print_favorite(&self) {
        println!("==================收藏课程==================");
        for course in &self.favorite_courses {
            println!(
                "教师: {:<10}课程: {:<20}ID: {:<30}类型: {:<10}",
                course.SKJS, course.KCM, course.JXBID, course.teaching_class_type
            );
        }
        println!("============================================");
    }

    // 抢课工作线程
    async fn work_thread(
        client: Client,
        token: String,
        batch_id: String,
        class_type: String,
        class_id: String,
        secret_val: String,
        name: String,
        current_status: Arc<Mutex<HashMap<String, String>>>,
        try_if_capacity_full: bool,
    ) {
        loop {
            let url = "https://icourses.jlu.edu.cn/xsxk/sc/clazz/addxk";
            let mut headers = HeaderMap::new();
            headers.insert("Authorization", HeaderValue::from_str(&token).unwrap());
            headers.insert("batchId", HeaderValue::from_str(&batch_id).unwrap());

            let mut params = HashMap::new();
            params.insert("clazzType", &class_type);
            params.insert("clazzId", &class_id);
            params.insert("secretVal", &secret_val);

            match client
                .post(url)
                .headers(headers)
                .query(&params)
                .send()
                .await
            {
                Ok(resp) => {
                    if let Ok(json) = resp.json::<serde_json::Value>().await {
                        let code = json["code"].as_i64().unwrap_or(0);
                        let msg = json["msg"].as_str().unwrap_or("");

                        let mut status = current_status.lock().unwrap();
                        if status.get(&class_id) == Some(&"doing".to_string()) {
                            if code == 200 {
                                println!("✅ 选课成功 [{}]", name);
                                status.insert(class_id.clone(), "done".to_string());
                                break;
                            } else if code == 500 {
                                match msg {
                                    "该课程已在选课结果中" => {
                                        println!("ℹ️ [{}] {}", name, msg);
                                        status.insert(class_id.clone(), "done".to_string());
                                        break;
                                    }
                                    "本轮次选课暂未开始" => {
                                        println!("⏰ [{}]本轮次选课暂未开始", name);
                                        continue;
                                    }
                                    "课容量已满" => {
                                        println!("😞 {}课容量已满", name);
                                        if !try_if_capacity_full {
                                            break;
                                        }
                                        continue;
                                    }
                                    "参数校验不通过" => {
                                        println!("❌ [{:?}]", json);
                                        continue;
                                    }
                                    _ => {
                                        println!("⚠️ [{}] {}", name, msg);
                                        continue;
                                    }
                                }
                            } else if code == 401 {
                                println!("🔐 {}", msg);
                                break;
                            } else {
                                println!("🔄 [{}]: 失败，重试中...", code);
                                continue;
                            }
                        } else {
                            break;
                        }
                    }
                }
                Err(e) => {
                    println!("🌐 请求错误: {}，重试中...", e);
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    continue;
                }
            }
        }
    }

    // 抢课主函数
    async fn fuck_my_favorite(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.get_favorite().await?;

        if !self.favorite_courses.is_empty() {
            let current_status: Arc<Mutex<HashMap<String, String>>> =
                Arc::new(Mutex::new(HashMap::new()));

            let mut tasks = Vec::new();

            for course in &self.favorite_courses {
                let status = Arc::clone(&current_status);
                status
                    .lock()
                    .unwrap()
                    .insert(course.JXBID.clone(), "doing".to_string());

                for _ in 0..WORK_THREAD_COUNT {
                    let client = self.client.clone();
                    let token = self.token.clone();
                    let batch_id = self.batch_id.clone();
                    let class_type = course.teaching_class_type.clone();
                    let class_id = course.JXBID.clone();
                    let secret_val = course.secret_val.clone().unwrap_or_default();
                    let name = course.KCM.clone();
                    let status = Arc::clone(&status);
                    let try_if_capacity_full = self.try_if_capacity_full;

                    tasks.push(tokio::spawn(Self::work_thread(
                        client,
                        token,
                        batch_id,
                        class_type,
                        class_id,
                        secret_val,
                        name,
                        status,
                        try_if_capacity_full,
                    )));
                }
            }

            join_all(tasks).await;
            println!("🎯 本轮抢课结束，继续检查...");
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 4 {
        println!("用法: {} 用户名 密码 批次ID <循环>", args[0]);
        return Ok(());
    }

    let username = args[1].clone();
    let password = args[2].clone();
    let batch_id: usize = args[3].parse()?;
    let mut debug_request_count = 0;

    loop {
        let mut icourses = ICourses::new(username.clone(), password.clone()).await?;

        // 无限重试登录
        while !icourses.login().await? {
            println!("🔄 登录失败，重试中...");
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        icourses.set_batch_id(batch_id).await?;

        // 启动WebSocket心跳（在后台运行）
        let icourses_clone = icourses.clone();
        let websocket_task = tokio::spawn(async move {
            if let Err(e) = icourses_clone.maintain_websocket_heartbeat().await {
                println!("💔 WebSocket心跳维护失败: {}", e);
            }
        });

        // 等待一小段时间确保WebSocket连接建立
        println!("⏳ 等待WebSocket连接建立...");
        tokio::time::sleep(Duration::from_millis(3000)).await;

        icourses.get_favorite().await?;
        icourses.print_favorite();
        icourses.fuck_my_favorite().await?;

        icourses.get_select().await?;
        icourses.print_select();
        debug_request_count += 1;
        println!("🔢 DEBUG_REQUEST_COUNT: {}\n", debug_request_count);

        if args.len() == 4 {
            // 在退出前终止WebSocket任务
            websocket_task.abort();
            break;
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    Ok(())
}
