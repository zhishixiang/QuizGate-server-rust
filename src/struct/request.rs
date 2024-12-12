use serde::{Deserialize, Serialize};

// 存放添加白名单的请求的结构体
#[derive(Serialize, Deserialize)]

pub struct Request {
    pub client_key: String,
    pub(crate) player_id: String,
}