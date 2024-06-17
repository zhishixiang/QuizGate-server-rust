// 存放添加白名单的请求的结构体
pub struct Request {
    pub client_key: String,
    pub(crate) player_id: String,
}