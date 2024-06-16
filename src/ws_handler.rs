use std::sync::Arc;
use futures_util::{StreamExt, SinkExt};
use rusqlite::Connection;
use tokio::sync::Mutex;
use crate::WsStream;

pub async fn ws_handler(ws_stream: WsStream, conn: Arc<Mutex<Connection>>) {
    let (mut write, mut read) = ws_stream.split();
    // 读取消息
    while let Some(msg) = read.next().await {
        match msg {
            Ok(msg) => {
                println!("{}", msg.clone().into_text().unwrap());
                conn.lock().execute("", ());
                if let Err(e) = write.send(msg).await {
                    eprintln!("Error sending message: {}", e);
                    break;
                }
            }
            Err(e) => {
                eprintln!("Error receiving message: {}", e);
                break;
            }
        }
    }
}