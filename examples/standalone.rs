#![allow(non_snake_case)]

use aes::cipher::{block_padding::Pkcs7, generic_array::GenericArray, BlockEncryptMut, KeyInit};
use aes::Aes128;
use futures::future::join_all;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

type Aes128EcbEnc = ecb::Encryptor<Aes128>;
const WORK_THREAD_COUNT: usize = 8;

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
        // let cipher = Aes128Cbc::new_from_slices(&self.aes_key, &[0u8; 16])?;
        // let encrypted = cipher.encrypt_vec(self.password.as_bytes());

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

        let resp = self
            .client
            .post(login_url)
            // .json(&params)
            .query(&params)
            .send()
            .await?;

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
            Ok(())
        } else {
            println!("获取已选课程失败: {}", resp_json["msg"]);
            Err("获取已选课程失败".into())
        }
    }

    // 获取收藏课程列表
    async fn get_favorite(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let url = "https://icourses.jlu.edu.cn/xsxk/sc/clazz/list";
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", HeaderValue::from_str(&self.token)?);
        headers.insert("batchId", HeaderValue::from_str(&self.batch_id)?);

        let resp = self.client.post(url).headers(headers).send().await?;

        let resp_json: serde_json::Value = resp.json().await?;

        if resp_json["code"] == 200 {
            self.favorite_courses = serde_json::from_value(resp_json["data"].clone())?;
            Ok(())
        } else {
            println!("获取收藏课程失败: {}", resp_json["msg"]);
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
                                println!("选课成功 [{}]", name);
                                status.insert(class_id.clone(), "done".to_string());
                                break;
                            } else if code == 500 {
                                match msg {
                                    "该课程已在选课结果中" => {
                                        println!("[{}] {}", name, msg);
                                        status.insert(class_id.clone(), "done".to_string());
                                        break;
                                    }
                                    "本轮次选课暂未开始" => {
                                        println!("[{}]本轮次选课暂未开始", name);
                                        continue;
                                    }
                                    "课容量已满" => {
                                        println!("{}课容量已满", name);
                                        if !try_if_capacity_full {
                                            break;
                                        }
                                        continue;
                                    }
                                    "参数校验不通过" => {
                                        println!("[{:?}]", json);
                                        // [Object {"code": Number(500), "data": Null, "msg": String("参数校验不通过")}]
                                        continue;
                                    }
                                    _ => {
                                        println!("[{}] {}", name, msg);
                                        continue;
                                    }
                                }
                            } else if code == 401 {
                                println!("{}", msg);
                                break;
                            } else {
                                println!("[{}]: 失败，重试中...", code);
                                continue;
                            }
                        } else {
                            break;
                        }
                    }
                }
                Err(e) => {
                    println!("请求错误: {}，重试中...", e);
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
            println!("本轮抢课结束，继续检查...");
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

    // 打印arg
    println!("args: {:?}", args);

    let username = args[1].clone();
    let password = args[2].clone();
    let batch_id: usize = args[3].parse()?;
    let mut debug_request_count = 0;

    loop {
        let mut icourses = ICourses::new(username.clone(), password.clone()).await?;

        // 无限重试登录
        while !icourses.login().await? {
            println!("登录失败，重试中...");
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        icourses.set_batch_id(batch_id).await?;
        icourses.get_favorite().await?;
        icourses.print_favorite();
        icourses.fuck_my_favorite().await?;

        icourses.get_select().await?;
        icourses.print_select();
        debug_request_count += 1;
        println!("DEBUG_REQUEST_COUNT: {}\n", debug_request_count);

        if args.len() == 4 {
            break;
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    Ok(())
}
