/// Common parameters for login requests
#[derive(Debug, Clone)]
pub struct LoginParams {
    pub username: String,
    pub encrypted_password: String,
    pub captcha: String,
    pub uuid: String,
}

/// Common parameters for course selection
#[derive(Debug, Clone)]
pub struct CourseSelectParams {
    pub token: String,
    pub batch_id: String,
    pub class_type: String,
    pub class_id: String,
    pub secret_val: String,
}

/// Common parameters for course queries
#[derive(Debug, Clone)]
pub struct CourseQueryParams {
    pub token: String,
    pub batch_id: String,
}
