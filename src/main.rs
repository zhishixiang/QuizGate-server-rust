mod database;
mod ws_handler;

use std::sync::Arc;
use futures_util::future::err;
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, tungstenite::protocol::Message, WebSocketStream};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

type WsStream = WebSocketStream<TcpStream>;

#[tokio::main]
async fn main() {
    if let Ok(conn) = database::new_database_conn().await{
        let conn = Arc::new(Mutex::new(conn));
        let addr = "127.0.0.1:9001";
        let listener = TcpListener::bind(&addr).await.unwrap();
        println!("Listening on: {}", addr);
        // 接受tcp链接
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(async move {
                // 升级为websocket链接
                match accept_async(stream).await {
                    // 升级成功
                    Ok(ws_stream) => ws_handler::ws_handler(ws_stream,conn.clone()),
                    // 升级失败
                    Err(e) => {
                        eprintln!("Error during WebSocket handshake: {}", e);
                    }
                }
            });
        }
    }else {
        panic!("读取数据库失败!")
    }
}

