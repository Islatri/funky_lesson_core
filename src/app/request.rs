//! No-WASM application implementation
//!
//! This module contains the application logic for non-WASM environments,
//! including both TUI and GUI implementations.

#[cfg(all(feature = "no-wasm", feature = "gui"))]
use crate::model::structs::EnrollmentStatus;
use crate::{
    client::request,
    crypto,
    error::{ErrorKind, Result},
};
use futures::future::join_all;
use reqwest::Client;
use std::{collections::HashMap, sync::Arc, time::Duration};

#[cfg(all(feature = "no-wasm", feature = "tui"))]
use std::sync::Mutex as StdMutex;
#[cfg(all(feature = "no-wasm", feature = "gui"))]
use tokio::sync::Mutex as TokioMutex;

use crate::model::structs::{BatchInfo, CourseInfo};

const WORK_THREAD_COUNT: usize = 4;

// GUI-specific functionality
#[cfg(all(feature = "no-wasm", feature = "gui"))]
pub mod gui {
    use super::*;

    pub async fn login(
        client: &Client,
        username: &str,
        password: &str,
        captcha: &str, // GUI模式下直接接收验证码
        uuid: &str,    // GUI模式下直接接收uuid
    ) -> Result<(String, Vec<BatchInfo>)> {
        // Get AES key
        let aes_key = request::get_aes_key(client).await?;

        // Encrypt password and login
        let encrypted_password = crypto::encrypt_password(password, &aes_key)?;
        let login_resp =
            request::send_login_request(client, username, &encrypted_password, captcha, uuid)
                .await?;

        if login_resp["code"] == 200 && login_resp["msg"] == "登录成功" {
            let token = login_resp["data"]["token"]
                .as_str()
                .ok_or_else(|| ErrorKind::ParseError("Invalid token".to_string()))?
                .to_string();

            let batch_list =
                serde_json::from_value(login_resp["data"]["student"]["electiveBatchList"].clone())?;

            Ok((token, batch_list))
        } else {
            Err(ErrorKind::ParseError(login_resp["msg"].to_string()).into())
        }
    }

    pub async fn get_captcha_inner(client: &Client) -> Result<(String, String)> {
        let (uuid, captcha_b64) = request::get_captcha(client).await?;
        let captcha_img = crypto::decode_captcha_image(&captcha_b64)?;
        let base64 = base64_simd::STANDARD;
        std::fs::write("captcha.png", &captcha_img)?;
        Ok((uuid, base64.encode_to_string(captcha_img)))
    }

    pub async fn enroll_courses(
        client: &Client,
        token: &str,
        batch_id: &str,
        courses: &[CourseInfo],
        try_if_capacity_full: bool,
        status: Arc<TokioMutex<EnrollmentStatus>>,
        should_continue: Arc<TokioMutex<bool>>,
    ) -> Result<()> {
        if courses.is_empty() {
            return Ok(());
        }

        let current_status: Arc<TokioMutex<HashMap<String, String>>> =
            Arc::new(TokioMutex::new(HashMap::new()));
        let mut tasks = Vec::new();
        let total_requests = Arc::new(TokioMutex::new(0u32));

        // 为每个课程创建一个任务
        for thread_id in 0..WORK_THREAD_COUNT {
            let client = client.clone();
            let token = token.to_string();
            let batch_id = batch_id.to_string();
            let courses = courses.to_vec();
            let status_map = Arc::clone(&current_status);
            let enrollment_status = Arc::clone(&status);
            let should_continue = Arc::clone(&should_continue);
            let total_requests = Arc::clone(&total_requests);
            let course_count = courses.len();

            tasks.push(tokio::spawn(async move {
                let mut course_idx = thread_id % course_count;

                while *should_continue.lock().await {
                    let course = &courses[course_idx];

                    // 更新状态
                    {
                        let mut counter = total_requests.lock().await;
                        *counter += 1;
                        let statuses: Vec<String> = {
                            let status_map = status_map.lock().await;
                            courses
                                .iter()
                                .map(|c| {
                                    format!(
                                        "[{}]{}",
                                        c.KCM,
                                        status_map.get(&c.JXBID).unwrap_or(&"等待中".to_string())
                                    )
                                })
                                .collect()
                        };

                        let mut status = enrollment_status.lock().await;
                        status.total_requests = *counter;
                        status.course_statuses = statuses;
                    }

                    // 尝试选课
                    let _result = course_enrollment_worker(
                        client.clone(),
                        token.clone(),
                        batch_id.clone(),
                        course.clone(),
                        Arc::clone(&status_map),
                        try_if_capacity_full,
                    )
                    .await;

                    if !*should_continue.lock().await {
                        break;
                    }

                    course_idx = (course_idx + 1) % course_count;

                    // 短暂延迟避免请求过快
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }));
        }

        join_all(tasks).await;
        Ok(())
    }

    async fn course_enrollment_worker(
        client: Client,
        token: String,
        batch_id: String,
        course: CourseInfo,
        status_map: Arc<TokioMutex<HashMap<String, String>>>,
        try_if_capacity_full: bool,
    ) -> Result<()> {
        let result = request::select_course(
            &client,
            &token,
            &batch_id,
            &course.teaching_class_type.clone().unwrap_or_default(),
            &course.JXBID,
            &course.secret_val.clone().unwrap_or_default(),
        )
        .await;

        match result {
            Ok(json) => {
                let code = json["code"].as_i64().unwrap_or(0);
                let msg = json["msg"].as_str().unwrap_or("");

                let status = match (code, msg) {
                    (200, _) => "选课成功",
                    (500, "该课程已在选课结果中") => "已选",
                    (500, "本轮次选课暂未开始") => "未开始",
                    (500, "课容量已满") if !try_if_capacity_full => "已满",
                    (500, "课容量已满") => "等待中",
                    (500, "参数校验不通过") => "参数错误",
                    (401, _) => "未登录",
                    _ => "失败",
                };

                status_map
                    .lock()
                    .await
                    .insert(course.JXBID.clone(), status.to_string());
                Ok(())
            }
            Err(e) => {
                status_map
                    .lock()
                    .await
                    .insert(course.JXBID.clone(), "请求错误".to_string());
                Err(e)
            }
        }
    }
}

// TUI-specific functionality
#[cfg(all(feature = "no-wasm", feature = "tui"))]
pub mod tui {
    use super::*;

    pub async fn login(
        client: &Client,
        username: &str,
        password: &str,
    ) -> Result<(String, Vec<BatchInfo>)> {
        // Get AES key
        let aes_key = request::get_aes_key(client).await?;

        // Get and save captcha
        let (uuid, captcha_b64) = request::get_captcha(client).await?;
        let captcha_img: Vec<u8> = crypto::decode_captcha_image(&captcha_b64)?;
        std::fs::write("captcha.png", captcha_img)?;

        // Get captcha input
        println!("Please check captcha.png and enter the captcha:");
        std::io::Write::flush(&mut std::io::stdout())?;
        let mut captcha = String::new();
        std::io::stdin().read_line(&mut captcha)?;
        let captcha = captcha.trim().to_string();

        // Encrypt password and login
        let encrypted_password = crypto::encrypt_password(password, &aes_key)?;
        let login_resp =
            request::send_login_request(client, username, &encrypted_password, &captcha, &uuid)
                .await?;

        if login_resp["code"] == 200 && login_resp["msg"] == "登录成功" {
            let token = login_resp["data"]["token"]
                .as_str()
                .ok_or_else(|| ErrorKind::ParseError("Invalid token".to_string()))?
                .to_string();

            let batch_list =
                serde_json::from_value(login_resp["data"]["student"]["electiveBatchList"].clone())?;

            print_login_success(&login_resp);
            Ok((token, batch_list))
        } else {
            println!("Login failed: {}", login_resp["msg"]);
            Err(ErrorKind::ParseError("Login failed".to_string()).into())
        }
    }

    pub async fn enroll_courses(
        client: &Client,
        token: &str,
        batch_id: &str,
        courses: &[CourseInfo],
        try_if_capacity_full: bool,
    ) -> Result<()> {
        if courses.is_empty() {
            return Ok(());
        }

        let current_status: Arc<StdMutex<HashMap<String, String>>> =
            Arc::new(StdMutex::new(HashMap::new()));
        let mut tasks = Vec::new();

        // 创建 WORK_THREAD_COUNT 个工作线程
        for thread_id in 0..WORK_THREAD_COUNT {
            let client = client.clone();
            let token = token.to_string();
            let batch_id = batch_id.to_string();
            let courses = courses.to_vec(); // 克隆整个课程列表
            let status = Arc::clone(&current_status);
            let course_count = courses.len();

            tasks.push(tokio::spawn(async move {
                // 从不同位置开始遍历课程
                let mut course_idx = thread_id % course_count;
                for _ in 0..course_count {
                    let course = &courses[course_idx];
                    let class_type = course.teaching_class_type.clone().unwrap_or_default();
                    let class_id = course.JXBID.clone();
                    let secret_val = course.secret_val.clone().unwrap_or_default();
                    let name = course.KCM.clone();

                    {
                        let mut status = status.lock().unwrap();
                        if !status.contains_key(&class_id) {
                            status.insert(class_id.clone(), "doing".to_string());
                        }
                    }

                    // 尝试选课
                    course_enrollment_worker(
                        client.clone(),
                        token.clone(),
                        batch_id.clone(),
                        class_type,
                        class_id,
                        secret_val,
                        name,
                        Arc::clone(&status),
                        try_if_capacity_full,
                    )
                    .await;

                    // 循环移动到下一个课程
                    course_idx = (course_idx + 1) % course_count;
                }
            }));
        }

        join_all(tasks).await;
        println!("本轮抢课结束，继续检查...");
        Ok(())
    }

    async fn course_enrollment_worker(
        client: Client,
        token: String,
        batch_id: String,
        class_type: String,
        class_id: String,
        secret_val: String,
        name: String,
        current_status: Arc<StdMutex<HashMap<String, String>>>,
        try_if_capacity_full: bool,
    ) {
        loop {
            // 检查课程状态
            {
                let status = current_status.lock().unwrap();
                if status.get(&class_id) != Some(&"doing".to_string()) {
                    break;
                }
            }

            let result = request::select_course(
                &client,
                &token,
                &batch_id,
                &class_type,
                &class_id,
                &secret_val,
            )
            .await;

            match result {
                Ok(json) => {
                    let code = json["code"].as_i64().unwrap_or(0);
                    let msg = json["msg"].as_str().unwrap_or("");

                    let mut status = current_status.lock().unwrap();
                    if status.get(&class_id) == Some(&"doing".to_string()) {
                        match (code, msg) {
                            (200, _) => {
                                println!("选课成功 [{name}]");
                                status.insert(class_id.clone(), "done".to_string());
                                break;
                            }
                            (500, "该课程已在选课结果中") => {
                                println!("[{name}] {msg}");
                                status.insert(class_id.clone(), "done".to_string());
                                break;
                            }
                            (500, "本轮次选课暂未开始") => {
                                println!("[{name}]本轮次选课暂未开始");
                                continue;
                            }
                            (500, "课容量已满") => {
                                println!("{name}课容量已满");
                                if !try_if_capacity_full {
                                    break;
                                }
                                continue;
                            }
                            (500, "参数校验不通过") => {
                                println!("[{json:?}]");
                                continue;
                            }
                            (401, _) => {
                                println!("{msg}");
                                break;
                            }
                            _ => {
                                println!("[{code}]: 失败，重试中...");
                                continue;
                            }
                        }
                    } else {
                        break;
                    }
                }
                Err(e) => {
                    println!("请求错误: {e}，重试中...");
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    continue;
                }
            }
        }
    }

    fn print_login_success(login_resp: &serde_json::Value) {
        if let Some(student) = login_resp["data"]["student"].as_object() {
            println!("Login success!");
            println!("=====================================");
            println!("XH: {}", student["XH"].as_str().unwrap_or(""));
            println!("XM: {}", student["XM"].as_str().unwrap_or(""));
            println!("ZYMC: {}", student["ZYMC"].as_str().unwrap_or(""));
            println!("=====================================");

            if let Some(batch_list) = student["electiveBatchList"].as_array() {
                for batch in batch_list {
                    if let Some(batch) = batch.as_object() {
                        println!("name: {}", batch["name"].as_str().unwrap_or(""));
                        println!("BeginTime: {}", batch["beginTime"].as_str().unwrap_or(""));
                        println!("EndTime: {}", batch["endTime"].as_str().unwrap_or(""));
                        println!("=====================================");
                    }
                }
            }
        }
    }

    pub fn print_batch_info(batch: &BatchInfo) {
        println!("Selected BatchId:");
        println!("=====================================");
        println!("name: {}", batch.name);
        println!("BeginTime: {}", batch.begin_time);
        println!("EndTime: {}", batch.end_time);
        println!("=====================================");
    }

    pub fn print_courses(selected: &[CourseInfo], favorite: &[CourseInfo]) {
        println!("==================已选课程==================");
        for course in selected {
            println!(
                "教师: {:<10}课程: {:<20}ID: {:<30}",
                course.SKJS, course.KCM, course.JXBID
            );
        }

        println!("==================收藏课程==================");

        for course in favorite {
            let teaching_class_type = course.teaching_class_type.clone().unwrap_or_default();
            println!(
                "教师: {:<10}课程: {:<20}ID: {:<30}类型: {:<10}",
                course.SKJS, course.KCM, course.JXBID, teaching_class_type
            );
        }
        println!("============================================");
    }
}

// Common functionality for both TUI and GUI
pub async fn set_batch(
    client: &Client,
    token: &str,
    batch_list: &[BatchInfo],
    batch_idx: usize,
) -> Result<String> {
    if batch_idx >= batch_list.len() {
        return Err(ErrorKind::ParseError("Invalid batch index".to_string()).into());
    }

    let batch_id = batch_list[batch_idx].code.clone();
    let resp = request::set_batch(client, &batch_id, token).await?;

    if resp["code"] != 200 {
        return Err(ErrorKind::ParseError("Failed to set batch".to_string()).into());
    }

    #[cfg(all(feature = "no-wasm", feature = "tui"))]
    tui::print_batch_info(&batch_list[batch_idx]);

    Ok(batch_id)
}

pub async fn get_courses(
    client: &Client,
    token: &str,
    batch_id: &str,
) -> Result<(Vec<CourseInfo>, Vec<CourseInfo>)> {
    let selected = request::get_selected_courses(client, token, batch_id).await?;
    let favorite = request::get_favorite_courses(client, token, batch_id).await?;

    let selected_courses: Vec<CourseInfo> = if selected["code"] == 200 {
        serde_json::from_value(selected["data"].clone())?
    } else {
        return Err(ErrorKind::CourseError(selected["msg"].to_string()).into());
    };

    let favorite_courses: Vec<CourseInfo> = if favorite["code"] == 200 {
        serde_json::from_value(favorite["data"].clone())?
    } else {
        return Err(ErrorKind::CourseError(favorite["msg"].to_string()).into());
    };

    Ok((selected_courses, favorite_courses))
}

// Re-export specific functionality based on enabled features
#[cfg(all(feature = "no-wasm", feature = "gui"))]
pub use gui::*;

#[cfg(all(feature = "no-wasm", feature = "tui"))]
pub use tui::*;
