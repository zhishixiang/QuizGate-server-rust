use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use actix_files::NamedFile;
use actix_web::{App, get, HttpRequest, HttpResponse, HttpServer, Result, web};
use std::path::Path;
use actix_web::web::to;
use serde::Deserialize;
use serde_json::{json, Value};
use crate::structs::submit::{SubmitRequest, SubmitResponse};


#[get("/{test_id}")]
async fn index() -> Result<NamedFile> {
    Ok(NamedFile::open(PathBuf::from("templates/exam.html"))?)
}
// 静态资源
async fn resources(req: HttpRequest) -> Result<NamedFile> {
    let mut path:PathBuf = PathBuf::from("resources/");
    let filename:String = req.match_info().query("filename").parse().unwrap();
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
    }else {
        HttpResponse::Ok().json(json!({
        "code": 404
        }))
    }
}
async fn submit(req_body: web::Json<SubmitRequest>) -> HttpResponse{
    // 获取post请求内容
    let answer = &req_body.answer;
    let player_id = &req_body.player_id;
    let test_id = &req_body.paper_id;
    let file_path = format!("tests/{}.json", test_id);
    let mut score = 0;
    let mut paper_info: Value = json!({});
    if Path::new(&file_path).exists() {
        let mut file = match File::open(&file_path) {
            Ok(file) => file,
            Err(_) => return HttpResponse::InternalServerError().json(json!({"code": 500})),
        };

        let mut contents = String::new();
        if let Err(_) = file.read_to_string(&mut contents) {
            return HttpResponse::InternalServerError().json(json!({"code": 500}));
        }

        paper_info = match serde_json::from_str(&contents) {
            Ok(json) => json,
            Err(_) => return HttpResponse::InternalServerError().json(json!({"code": 500})),
        };

        let questions = paper_info["questions"].as_array().unwrap();
        for (i, question) in questions.iter().enumerate() {
            if question["type"] == "multiple" {
                if answer[i] == question["correct"] {
                    score += question["score"][1].as_i64().unwrap();
                } else if answer[i].as_i64() < question["correct"].as_i64() && !answer[i].is_null() {
                    score += question["score"][0].as_i64().unwrap();
                }
            } else if question["type"] == "radio" {
                if answer[i] == question["correct"] {
                    score += question["score"].as_i64().unwrap();
                }
            }
        }
    } else {
        return HttpResponse::NotFound().json(json!({"code": 404}));
    }
    return if score > paper_info["pass"].as_i64().unwrap() {
        HttpResponse::Ok().json(SubmitResponse {
            score,
            pass: true,
        })
    } else {
        HttpResponse::Ok().json(SubmitResponse {
            score,
            pass: false,
        })
    }
}
pub fn new_actix_server(){
    let sys = actix_rt::System::new();
    sys.block_on(async {
        let server = HttpServer::new(|| {
            App::new()
                .service(index)
                .route("/resources/{filename:.*}",web::get().to(resources))
                .service(
                    web::scope("/api")
                        .route("/get_test/{filename:.*}",web::get().to(get_test))
                        .route("/submit",web::post().to(submit))
                )
        })
            .bind("127.0.0.1:8081")
            .expect("HTTP服务无法绑定端口")
            .run();
        println!("HTTP服务启动成功");
        server.await.expect("HTTP服务意外退出:");
    });

    sys.run().expect("HTTP服务意外退出:");
}