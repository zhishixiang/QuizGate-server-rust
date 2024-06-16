use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{accept_async, WebSocketStream};
use crate::structs::client::Client;

mod database;
mod ws_handler;
mod error;
mod structs;

type WsStream = WebSocketStream<TcpStream>;
type ClientList = Arc<RwLock<Vec<Client>>>;
#[tokio::main]
async fn main() {
    // 新建客户端列表以存储所有正在连接的客户端
    let client_list:ClientList = Arc::new(RwLock::new(vec![]));
    if let Ok(sql_pool) = database::new_sql_pool().await{
        let sql_pool = Arc::new(sql_pool);
        let addr = "127.0.0.1:9001";
        let listener = TcpListener::bind(&addr).await.unwrap();
        println!("Listening on: {}", addr);
        // 接受tcp链接
        while let Ok((stream, addr)) = listener.accept().await {
            println!("客户端{}请求连接",addr.ip());
            let sql_pool = Arc::clone(&sql_pool);
            let client_list = Arc::clone(&client_list);
            tokio::spawn(async move{
                // 升级为websocket链接
                match accept_async(stream).await {
                    // 升级成功
                    Ok(ws_stream) => ws_handler::ws_handler(ws_stream,sql_pool,client_list).await,

                    // 升级失败
                    Err(e) => Ok({
                        eprintln!("与客户端使用websocket握手时出错: {}", e);
                    })
                }
            });
        }
    }else {
        panic!("读取数据库失败!")
    }
}

