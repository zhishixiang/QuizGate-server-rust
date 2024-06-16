use std::fmt::Error;
use std::ptr::null;
use std::sync::Arc;
use futures_util::{StreamExt, SinkExt};
use tokio::sync::{mpsc, Mutex, RwLock};
use log::error;
use sqlx::{pool::Pool, sqlite::{Sqlite, SqlitePoolOptions}};
use sqlx::types::JsonValue::Null;
use tungstenite::Error::{ConnectionClosed, Io, Protocol};
use tungstenite::Message;
use crate::error::{ConnectionClosedError, NoSuchKeyError};
use crate::{Client, ClientList, WsStream};
use json;
use json::parse;
use crate::structs::respond::Respond;

pub async fn ws_handler(ws_stream: WsStream, sql_pool: Arc<Pool<Sqlite>>, mut client_list: ClientList) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
{
    let (mut write, mut read) = ws_stream.split();
    let (tx, rx) = mpsc::channel::<String>(4);
    //状态码定义：0为初始化，1为连接成功，只有为0时才进行验证流程
    let mut status_code:i8 = 0;
    let mut client_key = String::new();
    // 读取消息
    while let Some(msg) = read.next().await {
        match msg {
            Ok(msg) => match msg {
                Message::Text(msg) => {
                    if status_code == 0 {
                        // 获取密钥
                        let key = msg.to_string();
                        // 验证密钥是否存在
                        let result: Result<Option<(String,)>, sqlx::Error> = sqlx::query_as("SELECT name FROM server_info WHERE key = ?")
                            .bind(&key)
                            .fetch_optional(&*sql_pool)
                            .await;
                        match result {
                            Ok(Some(row)) => {
                                println!("客户端{}上线", row.0);
                                status_code = 1;
                                // 获取锁
                                let mut client_list = client_list.write().await;
                                // 将客户端信息推送至客户端列表
                                client_list.push(Client{client_key:key.clone(),client_handler:tx.clone()});
                                client_key = key;
                                // 释放锁
                                drop(client_list);
                                // 发送响应数据
                                let respond = Respond{code:0,msg:row.0};
                                let _ = write.send(Message::Binary(serde_json::to_vec(&respond).unwrap())).await;
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
                }
                Message::Ping(_) | Message::Pong(_) => {
                    // Ping and Pong messages can be handled here
                }
                Message::Close(_) => {
                    println!("Client closed the connection");
                    if status_code == 1{
                        remove_client(&mut client_list, client_key).await;
                    }
                    break;
                }
                _ => {}
            },
            Err(e) => match e {
                ConnectionClosed => {
                    println!("客户端断开链接");
                    if status_code == 1{
                        remove_client(&mut client_list, client_key).await;
                    }
                    break;
                }
                Io(ref err) if err.kind() == std::io::ErrorKind::ConnectionReset => {
                    println!("客户端异常退出");
                    if status_code == 1{
                        remove_client(&mut client_list, client_key).await;
                    }
                    break;
                }
                _ => {
                    println!("Other error: {}", e);
                    if status_code == 1{
                        remove_client(&mut client_list, client_key).await;
                    }
                    break;
                }
            },
        }
    }

    Ok(())
}

async fn remove_client(client_list: &mut ClientList, mut client_key: String) {
    let mut client_list = client_list.write().await;
    client_list.retain(|client| client.client_key != client_key);
    drop(client_list)
}
