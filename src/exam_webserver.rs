use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use actix_files::NamedFile;
use actix_web::{App, get, HttpRequest, HttpResponse, HttpServer, Result, web};
use std::path::Path;
use serde_json::{json, Value};

#[get("/{test_id}")]
async fn index(path: web::Path<(String)>) -> Result<NamedFile> {
    let test_id = path.into_inner();
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
    let mut path:PathBuf = PathBuf::from("tests/");
    let filename:String = req.match_info().query("filename").parse().unwrap();
    // 为文件加上后缀名
    path.push(filename+".json");
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
    }
    HttpResponse::Ok().json(json!({
        "code": 200,
        "data": test_info,
        "is_server_online": true
    }))
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