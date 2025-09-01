use serde::{Deserialize, Serialize};

// Common data structures used across all platforms
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
#[allow(non_snake_case)] // API字段名与服务器保持一致
pub struct CourseInfo {
    pub SKJS: String,  // 教师名
    pub KCM: String,   // 课程名
    pub JXBID: String, // 教学班ID
    #[serde(rename = "teachingClassType")]
    pub teaching_class_type: Option<String>,
    #[serde(default, rename = "secretVal")]
    pub secret_val: Option<String>,
}

    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    pub struct EnrollmentStatus {
        pub total_requests: u32,
        pub course_statuses: Vec<String>,
        pub is_running: bool,
    }
