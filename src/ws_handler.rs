use std::fmt::Error;
use std::sync::Arc;
use futures_util::{StreamExt, SinkExt};
use tokio::sync::Mutex;
use log::error;
use sqlx::{pool::Pool, sqlite::{Sqlite, SqlitePoolOptions}};
use tungstenite::Message;
use crate::error::{ConnectionClosedError, NoSuchKeyError};
use crate::WsStream;

pub async fn ws_handler(ws_stream: WsStream, sql_pool: Arc<Mutex<Pool<Sqlite>>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
{
    let (mut write, mut read) = ws_stream.split();

    // 读取消息
    while let Some(msg) = read.next().await {
        match msg {
            Ok(msg) => {
                let key = msg.to_string();
                let sql_pool = sql_pool.lock().await;
                let result: Result<Option<(String,)>, sqlx::Error> = sqlx::query_as("SELECT name FROM server_info WHERE key = ?")
                    .bind(&key)
                    .fetch_optional(&*sql_pool)
                    .await;

                match result {
                    Ok(Some(row)) => {
                        println!("客户端{}上线", row.0)
                    }
                    Ok(None) => {
                        eprintln!("客户端发送了无效的key，断开链接");
                        let _ = write.send(Message::Close(None)).await;
                        return Err(NoSuchKeyError.into());
                    }
                    Err(e) => {
                        error!("Failed to execute query: {}", e);
                        let _ = write.send(Message::Close(None)).await;
                        return Err(ConnectionClosedError.into());
                    }
                }
            }
            Err(e) => {
                eprintln!("Error receiving message: {}", e);
                let _ = write.send(Message::Close(None)).await;
                return Err(ConnectionClosedError.into());
            }
        }
    }

    Ok(())
}
