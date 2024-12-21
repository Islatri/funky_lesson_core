#![allow(non_snake_case)]

use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration
};
use reqwest::Client;
use tokio;
use futures::future::join_all;
use crate::{
    crypto,
    request,
    error::{Result, ErrorKind}
};

const WORK_THREAD_COUNT: usize = 8;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BatchInfo {
    pub code: String,
    pub name: String,
    #[serde(rename = "beginTime")]
    pub begin_time: String,
    #[serde(rename = "endTime")]
    pub end_time: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CourseInfo {
    pub SKJS: String,     // 教师名
    pub KCM: String,      // 课程名
    pub JXBID: String,    // 教学班ID
    #[serde(rename = "teachingClassType")]
    pub teaching_class_type: Option<String>,
    #[serde(default, rename = "secretVal")]
    pub secret_val: Option<String>,
}

pub async fn login(
    client: &Client,
    username: &str,
    password: &str
) -> Result<(String, Vec<BatchInfo>)> {
    // Get AES key
    let aes_key = request::get_aes_key(client).await?;
    
    // Get and save captcha
    let (uuid, captcha_b64) = request::get_captcha(client).await?;
    let captcha_img = crypto::decode_captcha_image(&captcha_b64)?;
    std::fs::write("captcha.png", captcha_img)?;
    
    // Get captcha input
    println!("Please check captcha.png and enter the captcha:");
    std::io::Write::flush(&mut std::io::stdout())?;
    let mut captcha = String::new();
    std::io::stdin().read_line(&mut captcha)?;
    let captcha = captcha.trim().to_string();

    // Encrypt password and login
    let encrypted_password = crypto::encrypt_password(password, &aes_key)?;
    let login_resp = request::send_login_request(
        client,
        username,
        &encrypted_password,
        &captcha,
        &uuid
    ).await?;

    if login_resp["code"] == 200 && login_resp["msg"] == "登录成功" {
        let token = login_resp["data"]["token"]
            .as_str()
            .ok_or_else(|| ErrorKind::ParseError("Invalid token".to_string()))?
            .to_string();
            
        let batch_list = serde_json::from_value(
            login_resp["data"]["student"]["electiveBatchList"].clone()
        )?;

        print_login_success(&login_resp);
        Ok((token, batch_list))
    } else {
        println!("Login failed: {}", login_resp["msg"]);
        Err(ErrorKind::ParseError("Login failed".to_string()).into())
    }
}

pub async fn set_batch(
    client: &Client,
    token: &str,
    batch_list: &[BatchInfo],
    batch_idx: usize
) -> Result<String> {
    if batch_idx >= batch_list.len() {
        return Err(ErrorKind::ParseError("Invalid batch index".to_string()).into());
    }

    let batch_id = batch_list[batch_idx].code.clone();
    let resp = request::set_batch(client, &batch_id, token).await?;

    if resp["code"] != 200 {
        return Err(ErrorKind::ParseError("Failed to set batch".to_string()).into());
    }

    print_batch_info(&batch_list[batch_idx]);
    Ok(batch_id)
}

pub async fn get_courses(
    client: &Client,
    token: &str,
    batch_id: &str
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

pub async fn enroll_courses(
    client: &Client,
    token: &str,
    batch_id: &str,
    courses: &[CourseInfo],
    try_if_capacity_full: bool
) -> Result<()> {
    if courses.is_empty() {
        return Ok(());
    }

    let current_status: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));
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
                ).await;

                // 循环移动到下一个课程
                course_idx = (course_idx + 1) % course_count;
            }
        }));
    }

    join_all(tasks).await;
    println!("本轮抢课结束，继续检查...");
    Ok(())
}
// pub async fn enroll_courses(
//     client: &Client,
//     token: &str,
//     batch_id: &str,
//     courses: &[CourseInfo],
//     try_if_capacity_full: bool
// ) -> Result<()> {
//     if courses.is_empty() {
//         return Ok(());
//     }

//     let current_status: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));

//     for course in courses {
//         let status = Arc::clone(&current_status);
//         status.lock().unwrap().insert(course.JXBID.clone(), "doing".to_string());

//         let mut tasks = Vec::new();
//         for _ in 0..WORK_THREAD_COUNT {
//             let client = client.clone();
//             let token = token.to_string();
//             let batch_id = batch_id.to_string();
//             let class_type = course.teaching_class_type.clone().unwrap_or_default();
//             let class_id = course.JXBID.clone();
//             let secret_val = course.secret_val.clone().unwrap_or_default();
//             let name = course.KCM.clone();
//             let status = Arc::clone(&status);

//             tasks.push(tokio::spawn(course_enrollment_worker(
//                 client,
//                 token,
//                 batch_id,
//                 class_type,
//                 class_id,
//                 secret_val,
//                 name,
//                 status,
//                 try_if_capacity_full,
//             )));
//         }

//         join_all(tasks).await;
//     }
    
//     println!("本轮抢课结束，继续检查...");
//     Ok(())
// }

async fn course_enrollment_worker(
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
            &secret_val
        ).await;

        match result {
            Ok(json) => {
                let code = json["code"].as_i64().unwrap_or(0);
                let msg = json["msg"].as_str().unwrap_or("");

                let mut status = current_status.lock().unwrap();
                if status.get(&class_id) == Some(&"doing".to_string()) {
                    match (code, msg) {
                        (200, _) => {
                            println!("选课成功 [{}]", name);
                            status.insert(class_id.clone(), "done".to_string());
                            break;
                        },
                        (500, "该课程已在选课结果中") => {
                            println!("[{}] {}", name, msg);
                            status.insert(class_id.clone(), "done".to_string());
                            break;
                        },
                        (500, "本轮次选课暂未开始") => {
                            println!("[{}]本轮次选课暂未开始", name);
                            continue;
                        },
                        (500, "课容量已满") => {
                            println!("{}课容量已满", name);
                            if !try_if_capacity_full {
                                break;
                            }
                            continue;
                        },
                        (500, "参数校验不通过") => {
                            println!("[{:?}]", json);
                            continue;
                        },
                        (401, _) => {
                            println!("{}", msg);
                            break;
                        },
                        _ => {
                            println!("[{}]: 失败，重试中...", code);
                            continue;
                        }
                    }
                } else {
                    break;
                }
            },
            Err(e) => {
                println!("请求错误: {}，重试中...", e);
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

fn print_batch_info(batch: &BatchInfo) {
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