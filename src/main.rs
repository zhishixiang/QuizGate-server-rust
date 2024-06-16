use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::{accept_async, WebSocketStream};

mod database;
mod ws_handler;
mod error;

type WsStream = WebSocketStream<TcpStream>;

#[tokio::main]
async fn main() {
    if let Ok(sql_pool) = database::new_sql_pool().await{
        let sql_pool = Arc::new(Mutex::new(sql_pool));
        let addr = "127.0.0.1:9001";
        let listener = TcpListener::bind(&addr).await.unwrap();
        println!("Listening on: {}", addr);
        // 接受tcp链接
        while let Ok((stream, addr)) = listener.accept().await {
            println!("客户端{}请求连接",addr.ip());
            let sql_pool = Arc::clone(&sql_pool);
            tokio::spawn(async move{
                // 升级为websocket链接
                match accept_async(stream).await {
                    // 升级成功
                    Ok(ws_stream) => ws_handler::ws_handler(ws_stream,sql_pool).await,

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

