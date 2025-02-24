use crate::r#struct::awl_type::Key;
use crate::sql_server::SqlServerHandle;
use actix_multipart::Multipart;
use actix_web::{web, HttpResponse};
use futures_util::{StreamExt, TryStreamExt};
use serde_json::{json, Value};
use std::fs::File;
use std::io::Write;

pub(crate) async fn upload(mut payload: Multipart, sql_server_handle: web::Data<SqlServerHandle>) -> HttpResponse {
    while let Ok(Some(mut field)) = payload.try_next().await {
        // 将接收的数据转换为文本
        let mut text = String::new();
        while let Some(chunk) = field.next().await {
            match chunk {
                Ok(data) => {
                    match std::str::from_utf8(&data) {
                        Ok(data_str) => text.push_str(data_str),
                        Err(_) => {
                            return HttpResponse::BadRequest().json(json!({
                                "code": 400,
                                "message": "Invalid UTF-8 data"
                            }));
                        }
                    }
                }
                Err(_) => {
                    return HttpResponse::InternalServerError().json(json!({
                        "code": 500,
                        "message": "Internal server error"
                    }));
                }

            }
        }

        // 检测内容是否为json格式
        if let Ok(json) = serde_json::from_str::<Value>(&text) {
            // 验证json格式是否正确
            if !json.is_object() || !json.as_object().unwrap().contains_key("questions") {
                return HttpResponse::BadRequest().json(json!({"code": 400}));
            }
            // 读取客户端密钥
            let key: Key = json["client_key"].as_str().unwrap().to_string();
            if key.is_empty() {
                return HttpResponse::BadRequest().json(json!({"code": 400}));
            }
            let result = sql_server_handle.get_client_id(key).await;
            match result {
                Ok(id) => {
                    let file_path = format!("tests/{}.json", id);
                    let mut file = File::create(file_path).unwrap();
                    while let Some(chunk) = field.next().await {
                        let data = chunk.unwrap();
                        file.write_all(&data).unwrap();
                    }
                    return HttpResponse::Ok().json(json!({"code": 200}))
                }
                Err(e) => return HttpResponse::Forbidden().json(json!({"code": e.to_string()})),
            }
        } else {
            return HttpResponse::BadRequest().json(json!({"code": 400}))
        }
    }
    HttpResponse::InternalServerError().json(json!({"code": 500}))
}