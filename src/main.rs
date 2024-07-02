use std::sync::{Arc};
use std::thread::spawn;

use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::{accept_async, WebSocketStream};

use crate::exam_webserver::new_actix_server;
use crate::message_queue::create_mq_thread;
use crate::structs::client::Client;
use crate::structs::request::Request;

mod database;
mod ws_handler;
mod error;
mod structs;
mod exam_webserver;
mod message_queue;

type WsStream = WebSocketStream<TcpStream>;
type ClientList = Arc<Mutex<Vec<Client>>>;
// type Queue = Arc<Mutex<MessageQueue>>;
#[derive(Serialize, Deserialize)]
struct MessageQueue {
    messages: Vec<Request>,
}
#[tokio::main]
async fn main() {
    // 新建客户端列表以存储所有正在连接的客户端
    let client_list: ClientList = Arc::new(Mutex::new(vec![]));
    // 新建sql连接池
    if let Ok(sql_pool) = database::new_sql_pool().await {
        let sql_pool = Arc::new(sql_pool);
        let addr = "127.0.0.1:9001";
        let listener = TcpListener::bind(&addr).await.unwrap();
        println!("Listening on: {}", addr);
        // 创建 HttpServer 实例并配置服务
        let client_list_actix = Arc::clone(&client_list);
        let _actix_thread = spawn(move || {
            new_actix_server(client_list_actix);
        });
        // 创建消息队列线程
        /*
        spawn(move || {
            create_mq_thread(client_list_mq);
        });
        */
        // 接受tcp链接
        while let Ok((stream, addr)) = listener.accept().await {
            println!("客户端{}请求连接", addr.ip());
            let sql_pool = Arc::clone(&sql_pool);
            let client_list_ws = Arc::clone(&client_list);
            tokio::spawn(async move {
                // 升级为websocket链接
                match accept_async(stream).await {
                    // 升级成功
                    Ok(ws_stream) => ws_handler::ws_handler(ws_stream, sql_pool, client_list_ws).await,

                    // 升级失败
                    Err(e) => Ok({
                        eprintln!("与客户端使用websocket握手时出错: {}", e);
                    })
                }
            });
        }
    } else {
        panic!("读取数据库失败!")
    }
}

