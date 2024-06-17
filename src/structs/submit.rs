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
pub struct SubmitResponse {
    pub(crate) score: i64,
    pub(crate) pass: bool,
}