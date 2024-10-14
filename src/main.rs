#![allow(unused_assignments)]

use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::path::PathBuf;

pub use crate::structs::submit::{SubmitRequest, SubmitResponse};
use crate::ws_server::{WsServer, WsServerHandle};
use actix_files::NamedFile;
use actix_multipart::Multipart;
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer, Result};
use database::SqlServer;
use futures_util::{StreamExt, TryStreamExt};
use serde_json::{json, Value};
use structs::awl_type::{Key, SqlFile};
use std::path::Path;
use std::sync::Arc;

use crate::utils::{mark, read_file};
use tokio::task::{spawn, spawn_local};

mod database;
mod error;
mod structs;
mod utils;
mod ws_handler;
mod ws_server;


async fn index() -> Result<NamedFile> {
    Ok(NamedFile::open(PathBuf::from("templates/exam.html"))?)
}

async fn upload_page() -> Result<NamedFile> {
    Ok(NamedFile::open(PathBuf::from("templates/upload.html"))?)
}

async fn register_page() -> Result<NamedFile> {
    Ok(NamedFile::open(PathBuf::from("templates/register.html"))?)
}
// 静态资源
async fn resources(req: HttpRequest) -> Result<NamedFile> {
    let mut path: PathBuf = PathBuf::from("resources/");
    let filename: String = req.match_info().query("filename").parse()?;
    path.push(filename);
    Ok(NamedFile::open(path)?)
}

// 获取试题内容
async fn get_test(req: HttpRequest) -> HttpResponse {
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
async fn submit(
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

// 将通过post上传的文件存入tests目录
async fn upload(mut payload: Multipart, ws_server: web::Data<WsServerHandle>) -> HttpResponse {
    while let Ok(Some(mut field)) = payload.try_next().await {
        // 将接收的数据转换为文本
        let mut text = String::new();
        while let Some(chunk) = field.next().await {
            let data = chunk.unwrap();
            text += std::str::from_utf8(&data).unwrap();
        }

        // 检测内容是否为json格式
        return if let Ok(json) = serde_json::from_str::<Value>(&text) {
            // 验证json格式是否正确
            if !json.is_object() || !json.as_object().unwrap().contains_key("questions") {
                return HttpResponse::BadRequest().json(json!({"code": 400}));
            }
            // 读取客户端密钥
            let key: Key = json["client_key"].as_str().unwrap().to_string();
            let result = ws_server.get_client_id(key).await;
            match result {
                Ok(id) => {
                    let file_path = format!("tests/{}.json", id);
                    let mut file = File::create(file_path).unwrap();
                    while let Some(chunk) = field.next().await {
                        let data = chunk.unwrap();
                        file.write_all(&data).unwrap();
                    }
                    HttpResponse::Ok().json(json!({"code": 200}))
                }
                Err(_) => HttpResponse::InternalServerError().json(json!({"code": 500})),
            }
        } else {
            HttpResponse::BadRequest().json(json!({"code": 400}))
        }
    }
    HttpResponse::InternalServerError().json(json!({"code": 500}))
}



async fn handle_ws_connection(
    req: HttpRequest,
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
    let address = "127.0.0.1:8081";
    let sql_file:SqlFile = "data.db".to_string();

    if let Ok((sql_server,sql_server_tx)) = SqlServer::new(sql_file).await {

        let (ws_server, ws_server_tx) = WsServer::new(sql_server_tx);

        let _ws_server = spawn(ws_server.run());

        let server = HttpServer::new(move || {
            App::new()
                .app_data(web::Data::new(ws_server_tx.clone()))
                .service(web::resource("/ws").route(web::get().to(handle_ws_connection)))
                .service(web::resource("/upload").route(web::get().to(upload_page)))
                .service(web::resource("/register").route(web::get().to(register_page)))
                .service(web::resource("/").route(web::get().to(index)))
                .service(web::resource("/{test_id}").route(web::get().to(index)))
                .route("/resources/{filename:.*}", web::get().to(resources))
                .service(
                    web::scope("/api")
                        .route("/get_test/{filename:.*}", web::get().to(get_test))
                        .route("/upload", web::post().to(upload))
                        .route("/submit", web::post().to(submit)),
                )
        })
        .workers(2)
        .bind(address)
        .expect("HTTP服务无法绑定端口")
        .run();
        env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

        log::info!("starting HTTP server at http://{address}");
        server.await.expect("HTTP服务意外退出:");
        Ok(())
    } else {
        panic!("读取数据库失败!")
    }
}
