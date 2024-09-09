use std::fs::File;
use std::io;
use std::io::Read;
use std::path::PathBuf;

use actix_files::NamedFile;
use actix_web::{App, get, HttpRequest, HttpResponse, HttpServer, Result, web, Error};
use std::path::Path;
use std::sync::Arc;
use futures_util::StreamExt;
use serde_json::{json, Value};

pub use crate::structs::submit::{SubmitRequest, SubmitResponse};
use crate::ws_server::{WsServer, WsServerHandle};

use tokio::{
    task::{spawn, spawn_local},
};

mod structs;
mod ws_handler;
mod ws_server;
mod database;

// 连接ID
pub type ConnId = u32;
// 客户端发送的验证密钥
pub type Key = String;
// 通过考试的玩家ID
pub type PlayerId = String;
#[get("/{test_id}")]
async fn index() -> Result<NamedFile> {
    Ok(NamedFile::open(PathBuf::from("templates/exam.html"))?)
}

// 静态资源
async fn resources(req: HttpRequest) -> Result<NamedFile> {
    let mut path: PathBuf = PathBuf::from("resources/");
    let filename: String = req.match_info().query("filename").parse().unwrap();
    path.push(filename);
    Ok(NamedFile::open(path)?)
}

// 获取试题内容
async fn get_test(req: HttpRequest) -> HttpResponse {
    let mut test_info: Value = json!({});
    let mut path: PathBuf = PathBuf::from("tests/");
    let filename: String = req.match_info().query("filename").parse().unwrap();
    // 为文件加上后缀名
    path.push(filename + ".json");
    if Path::new(&path).exists() {
        let mut file = match File::open(&path) {
            Ok(file) => file,
            Err(_) => return HttpResponse::NotFound().json(json!({"code": 404})),
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

async fn submit(req_body: web::Json<SubmitRequest>, ws_server: web::Data<WsServerHandle>) -> HttpResponse {
    // 获取post请求内容
    let answer = &req_body.answer;
    let player_id = &req_body.player_id;
    let test_id = &req_body.paper_id;
    let file_path = format!("tests/{}.json", test_id);
    let mut score = 0;
    let mut paper_info: Value = json!({});
    // 检测文件是否存在
    if Path::new(&file_path).exists() {
        let mut file = match File::open(&file_path) {
            Ok(file) => file,
            Err(_) => return HttpResponse::InternalServerError().json(json!({"code": 500})),
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
    // 返回分数和是否及格并直接通知客户端
    // 消息队列真tm难整，先不整了

    if score >= paper_info["pass"].as_i64().unwrap() {
        pass = true;
        let key:Key = paper_info["client_key"].as_str().unwrap().to_string();
        ws_server.send_message(key,player_id).await;
        /*
        for client in client_list.lock().await.deref(){
            if client.client_key == paper_info["client_key"] {
                client.client_handler.send(player_id.clone()).await.expect("TODO: panic message");
            }
        }
         */
    }

        // write_message_to_json_file(Request { client_key: paper_info["client_key"].to_string(), player_id: player_id.to_string() }).expect("写入消息队列失败");
    HttpResponse::Ok().json(SubmitResponse {
        score,
        pass,
    })

}

// 干得好，我要给你打易佰昏！
fn mark(answer: &Vec<Value>, paper_info: &Value) -> i64{
    let mut score:i64 = 0;
    let questions = paper_info["questions"].as_array().unwrap();
    for (i, question) in questions.iter().enumerate() {
        // 多选题
        if question["type"] == "multiple" {
            // 回答完全正确
            if answer[i] == question["correct"] {
                score += question["score"][1].as_i64().unwrap();
                // 部分内容正确且回答不为空
            } else if answer[i].as_i64() < question["correct"].as_i64() && !answer[i].is_null() {
                score += question["score"][0].as_i64().unwrap();
            }
            // 单选题
        } else if question["type"] == "radio" {
            if answer[i] == question["correct"] {
                score += question["score"].as_i64().unwrap();
            }
        }
    }
    score
}

async fn handle_ws_connection(req: HttpRequest,
                              stream: web::Payload,
                              ws_server: web::Data<WsServerHandle>,
) -> Result<HttpResponse, Error> {
    let (res, session, msg_stream) = actix_ws::handle(&req, stream)?;
    // 新建一个websocket handler
    spawn_local(ws_handler::chat_ws(
        (**ws_server).clone(),
        session,
        msg_stream,
    ));

    Ok(res)
}
// 启动actix服务
#[tokio::main(flavor = "current_thread")]
async fn main() -> io::Result<()> {
    if let Ok(sql_pool) = database::new_sql_pool().await {
        let sql_pool = Arc::new(sql_pool);

        let (chat_server, server_tx) = WsServer::new(sql_pool);

        let chat_server = spawn(chat_server.run());

        let server = HttpServer::new(move || {
            App::new()
                .app_data(web::Data::new(server_tx.clone()))
                .service(web::resource("/ws").route(web::get().to(handle_ws_connection)))
                .service(index)
                .route("/resources/{filename:.*}", web::get().to(resources))
                .service(
                    web::scope("/api")
                        .route("/get_test/{filename:.*}", web::get().to(get_test))
                    .route("/submit", web::post().to(submit))
                )
        })
            .workers(2)
            .bind("127.0.0.1:8081")
            .expect("HTTP服务无法绑定端口")
            .run();
        env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

        log::info!("starting HTTP server at http://localhost:8080");
        server.await.expect("HTTP服务意外退出:");
        Ok(())
    } else {
        panic!("读取数据库失败!")
    }
}