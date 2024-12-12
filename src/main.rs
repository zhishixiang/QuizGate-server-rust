#![allow(unused_assignments)]

use std::{fs, io};

pub use crate::r#struct::submit::{SubmitRequest, SubmitResponse};
use crate::ws_server::{WsServer, WsServerHandle};
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer, Result};
use database::SqlServer;
use r#struct::awl_type::SqlFile;
use tokio::task::{spawn, spawn_local};
use lazy_static::lazy_static;
use toml::Value;
use crate::email_server::{EmailServer};
use crate::service::{register, upload, resources, pages, test};

mod database;
mod error;
mod r#struct;
mod utils;
mod ws_handler;
mod ws_server;
mod email_server;
mod service;

// 读取配置文件config.toml并初始化全局变量
pub struct Config {
    pub local: bool,
    pub local_key: String,
}

lazy_static! {
        pub static ref CONFIG: Config = {
            let config_content = fs::read_to_string("config.toml").expect("无法读取配置文件！");
            let config: Value = toml::from_str(&config_content).expect("配置文件格式错误，请再次确认！");
            Config {
                local: config["local"].as_bool().expect("无法读取local字段！"),
                local_key: config["local_key"].as_str().expect("无法读取local_key字段！").to_string(),
            }
        };
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
    // 读取配置文件config.toml
    struct Config {
        local: bool,
        local_key: String,
    }
    
    lazy_static! {
        static ref CONFIG: Config = {
            let config_content = fs::read_to_string("config.toml").expect("无法读取配置文件！");
            let config: Value = toml::from_str(&config_content).expect("配置文件格式错误，请再次确认！");
            Config {
                local: config["local"].as_bool().expect("无法读取local字段！"),
                local_key: config["local_key"].as_str().expect("无法读取local_key字段！").to_string(),
            }
        };
    }

    if let Ok((sql_server,sql_server_tx)) = SqlServer::new(sql_file).await {
        // 启动线程
        let (ws_server, ws_server_tx) = WsServer::new(sql_server_tx.clone());
        
        let (email_server, email_server_tx) = EmailServer::new();
        
        let _ws_server = spawn(ws_server.run());

        let _sql_server = spawn(sql_server.run());

        let _email_server = spawn(email_server.run());

        // 启动HTTP服务
        let server = HttpServer::new(move || {
            App::new()
                .app_data(web::Data::new(ws_server_tx.clone()))
                .app_data(web::Data::new(email_server_tx.clone()))
                .app_data(web::Data::new(sql_server_tx.clone()))
                .service(web::resource("/ws").route(web::get().to(handle_ws_connection)))
                .service(web::resource("/upload").route(web::get().to(pages::upload_page)))
                .service(web::resource("/register").route(web::get().to(pages::register_page)))
                .service(web::resource("/").route(web::get().to(pages::index)))
                .service(web::resource("/verify/{token}").route(web::get().to(register::verify)))
                .service(web::resource("/{test_id}").route(web::get().to(pages::index)))
                .route("/resources/{filename:.*}", web::get().to(resources::resources))
                .service(
                    web::scope("/api")
                        .route("/get_test/{filename:.*}", web::get().to(test::get_test))
                        .route("/upload", web::post().to(upload::upload))
                        .route("/submit", web::post().to(test::submit))
                        .route("/register", web::post().to(register::register_pending)),
                )
        })
        .workers(2)
        .bind(address)
        .expect("端口被占用，无法启动HTTP服务！")
        .run();
        env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

        log::info!("starting HTTP server at http://{address}");
        server.await.expect("HTTP服务意外退出:");
        Ok(())
    } else {
        panic!("服务启动失败！");
    }
}
