use std::sync::Arc;
use futures_util::{StreamExt, SinkExt};
use rusqlite::{Connection, OptionalExtension, params};
use tokio::sync::Mutex;
use log::error;
use rusqlite::Error::QueryReturnedNoRows;
use crate::error::{ConnectionClosedError, NoSuchKeyError};
use crate::WsStream;

pub async fn ws_handler(ws_stream: WsStream, conn: Arc<Mutex<Connection>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
{
    let (mut write, mut read) = ws_stream.split();

    // 读取消息
    while let Some(msg) = read.next().await {
        match msg {
            Ok(msg) => {
                let key = msg.to_string();
                println!("{}", msg.clone().into_text().unwrap());

                let conn = conn.lock().await;
                let mut stmt = match conn.prepare("SELECT name FROM server_info WHERE key=?") {
                    Ok(stmt) => stmt,
                    Err(e) => {
                        error!("Failed to prepare statement: {}", e);
                        return Err(ConnectionClosedError.into());
                    }
                };

                let result: Result<String, _> = stmt.query_row(params![key], |row| row.get(0));
                match result {
                    Ok(name) => {
                        println!("Name: {:?}", name);
                    }
                    Err(QueryReturnedNoRows) => {
                        eprintln!("客户端发送了一个无效的key，断开链接");
                        return Err(ConnectionClosedError.into());
                    }
                    Err(e) => {
                        error!("Failed to execute query: {}", e);
                        return Err(ConnectionClosedError.into());
                    }
                }
            }
            Err(e) => {
                eprintln!("Error receiving message: {}", e);
                return Err(ConnectionClosedError.into());
            }
        }
    }

    Ok(())
}
