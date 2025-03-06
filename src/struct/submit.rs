use serde::{Deserialize, Serialize};
use serde_json::Value;

// 对于提交的试卷进行解析和响应的结构体
#[derive(Deserialize)]
pub struct SubmitRequest {
    pub(crate) answer: Vec<Value>,
    pub(crate) player_id: String,
    pub(crate) paper_id: String,
}

#[derive(Serialize)]
pub struct SubmitResponsePass {
    pub(crate) score: i64,
    pub(crate) pass: bool,
    pub(crate) count: u32,
}

#[derive(Serialize)]
pub struct SubmitResponseFail {
    pub(crate) score: i64,
    pub(crate) pass: bool,
}

#[derive(Deserialize,Debug)]
pub struct RegisterRequest {
    pub(crate) email: String,
    pub(crate) server_name: String,
    pub(crate) captcha_token: String,
}

#[derive(Deserialize, Debug)]
pub struct CaptchaResponse {
    pub(crate) success: bool,
    #[serde(rename = "error-codes")]
    pub(crate) error_codes: Option<Vec<String>>,

}