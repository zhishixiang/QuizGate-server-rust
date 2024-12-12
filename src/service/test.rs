use std::io::Read;
use std::path::Path;
use actix_web::{web, HttpRequest, HttpResponse};
use serde_json::{json, Value};
use crate::r#struct::awl_type::Key;
use crate::{SubmitRequest, SubmitResponse};
use crate::utils::{mark, read_file};
use crate::ws_server::WsServerHandle;

// 获取试题内容
pub(crate) async fn get_test(req: HttpRequest) -> HttpResponse {
    let mut test_info: Value = json!({});
    let filename: String = req.match_info().query("filename").parse().unwrap();
    // 如果get参数不为数字则返回错误
    match filename.parse::<i32>() {
        Ok(_) => {}
        Err(_) => return HttpResponse::BadRequest().body("Invalid file path"),
    }
    // 为文件加上后缀名
    let file_path = format!("tests/{}.json", filename);
    if Path::new(&file_path).exists() {
        let mut file = match read_file(&file_path) {
            Ok(value) => value,
            Err(error) => {
                log::error!("读取文件时出现错误：{error}");
                return HttpResponse::InternalServerError().json(json!({"code": 500}));
            }
        };

        let mut contents = String::new();
        if let Err(_) = file.read_to_string(&mut contents) {
            return HttpResponse::InternalServerError().json(json!({"code": 500}));
        }

        test_info = match serde_json::from_str(&contents) {
            Ok(json) => json,
            Err(_) => return HttpResponse::InternalServerError().json(json!({"code": 500})),
        };
        // 移除不该出现的部分
        test_info.as_object_mut().unwrap().remove("pass");
        test_info.as_object_mut().unwrap().remove("client_key");
        if let Some(questions) = test_info["questions"].as_array_mut() {
            for question in questions {
                question.as_object_mut().unwrap().remove("correct");
                question.as_object_mut().unwrap().remove("score");
            }
        }
        HttpResponse::Ok().json(json!({
        "code": 200,
        "data": test_info,
        "is_server_online": true
        }))
    } else {
        HttpResponse::Ok().json(json!({"code": 404}))
    }
}

// 提交试卷并进行打分
pub(crate) async fn submit(
    req_body: web::Json<SubmitRequest>,
    ws_server: web::Data<WsServerHandle>,
) -> HttpResponse {
    // 获取post请求内容
    let answer = &req_body.answer;
    let player_id = &req_body.player_id;
    let test_id = &req_body.paper_id;
    let file_path = format!("tests/{}.json", test_id);
    let mut score = 0;
    let mut paper_info: Value = json!({});
    // 检测文件是否存在
    if Path::new(&file_path).exists() {
        let mut file = match read_file(&file_path) {
            Ok(value) => value,
            Err(error) => {
                log::error!("读取文件时出现错误：{error}");
                return HttpResponse::InternalServerError().json(json!({"code": 500}));
            }
        };
        // 从文件读取问卷信息
        let mut contents = String::new();
        if let Err(_) = file.read_to_string(&mut contents) {
            return HttpResponse::InternalServerError().json(json!({"code": 500}));
        }
        // 转换为json文件
        paper_info = match serde_json::from_str(&contents) {
            Ok(json) => json,
            Err(_) => return HttpResponse::InternalServerError().json(json!({"code": 500})),
        };
        // 进行评分
        score = mark(answer, &paper_info);
    } else {
        return HttpResponse::NotFound().json(json!({"code": 404}));
    }
    let mut pass = false;

    if score >= paper_info["pass"].as_i64().unwrap() {
        pass = true;
        let key: Key = paper_info["client_key"].as_str().unwrap().to_string();
        ws_server.send_message(key, player_id).await;
    }
    HttpResponse::Ok().json(SubmitResponse { score, pass })
}