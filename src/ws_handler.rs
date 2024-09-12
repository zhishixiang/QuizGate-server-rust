use std::{
    pin::pin,
    time::{Duration, Instant},
};
use actix_ws::AggregatedMessage;
use futures_util::{
    future::{select, Either},
    StreamExt as _,
};
use serde_json::json;
use tokio::{sync::mpsc, time::interval};
use crate::{ConnId, Key};
use crate::error::{DuplicateConnectionsError, NoSuchKeyError};
use crate::ws_server::WsServerHandle;

/// 心跳包发送频率
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

/// 超时时间
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

/// Echo text & binary messages received from the client, respond to ping messages, and monitor
/// connection health to detect network issues and free up resources.
pub async fn chat_ws(
    chat_server: WsServerHandle,
    mut session: actix_ws::Session,
    msg_stream: actix_ws::MessageStream,
) {

    let mut last_heartbeat = Instant::now();
    let first_connect = Instant::now();
    let mut interval = interval(HEARTBEAT_INTERVAL);
    // 当前链接的验证状态
    let mut verified = false;

    let (conn_tx, mut conn_rx) = mpsc::unbounded_channel();

    let conn_id = chat_server.connect(conn_tx).await.unwrap();
    log::info!("新建链接{}",conn_id);
    let msg_stream = msg_stream
        .max_frame_size(128 * 1024)
        .aggregate_continuations()
        .max_continuation_size(2 * 1024 * 1024);

    let mut msg_stream = pin!(msg_stream);

    let close_reason = loop {
        // 钉住tick和rx以使用select来判断计数器和指令
        // 我也不知道为什么要这么写，不过原文是这么写的，还是保留了
        let tick = pin!(interval.tick());
        let msg_rx = pin!(conn_rx.recv());

        // TODO: nested select is pretty gross for readability on the match
        let messages = pin!(select(msg_stream.next(), msg_rx));
        match select(messages, tick).await {
            // 从客户端接受指令
            Either::Left((Either::Left((Some(Ok(msg)), _)), _)) => {
                log::debug!("msg: {msg:?}");

                match msg {
                    AggregatedMessage::Ping(bytes) => {
                        last_heartbeat = Instant::now();
                        // unwrap:
                        session.pong(&bytes).await.unwrap();
                    }

                    AggregatedMessage::Pong(_) => {
                        last_heartbeat = Instant::now();
                    }

                    AggregatedMessage::Text(text) => {
                        // 如果当前客户端未验证就一直处于验证状态
                        if !verified {
                            verified = process_text_msg(&chat_server, &mut session, &text, conn_id)
                                .await;
                        }
                    }

                    AggregatedMessage::Binary(_bin) => {
                        log::warn!("unexpected binary message");
                    }

                    AggregatedMessage::Close(reason) => break reason,
                }
            }

            // client WebSocket stream error
            Either::Left((Either::Left((Some(Err(err)), _)), _)) => {
                log::error!("{}", err);
                break None;
            }

            // client WebSocket stream ended
            Either::Left((Either::Left((None, _)), _)) => break None,

            // chat messages received from other room participants
            Either::Left((Either::Right((Some(chat_msg), _)), _)) => {
                let template = json!({
                    "code": 2,
                    "msg": chat_msg
                });
                session.text(template.to_string()).await.unwrap();
            }

            // all connection's message senders were dropped
            Either::Left((Either::Right((None, _)), _)) => unreachable!(
                "all connection message senders were dropped; chat server may have panicked"
            ),

            // 心跳包和验证超时判断
            Either::Right((_inst, _)) => {
                // 如果长时间未收到心跳包则断开链接
                if Instant::now().duration_since(last_heartbeat) > CLIENT_TIMEOUT {
                    log::info!(
                        "客户端{conn_id}在{CLIENT_TIMEOUT:?}秒内未发送心跳包，断开链接"
                    );
                    break None;
                } else if !verified && Instant::now().duration_since(first_connect) > CLIENT_TIMEOUT {
                    log::info!(
                        "{conn_id}在{CLIENT_TIMEOUT:?}秒内未进行验证，断开链接"
                    );
                    break None;
                }

                // send heartbeat ping
                let _ = session.ping(b"").await;
            },
        };
    };

    chat_server.disconnect(conn_id);
    // attempt to close connection gracefully
    let _ = session.close(close_reason).await;
    log::info!("客户端{}断开链接",conn_id);
}

async fn process_text_msg(
    chat_server: &WsServerHandle,
    session: &mut actix_ws::Session,
    text: &str,
    conn: ConnId,
) -> bool{
    let packet_recv: Result<serde_json::Value, serde_json::Error> = serde_json::from_str(text);
    match packet_recv {
        Ok(json) => {
            // 通过键来获取值
            let _code = json["code"].as_i64();  // 获取 "code" 的值
            let key:Key = json["key"].as_str().unwrap().to_string();    // 获取 "key" 的值
            match chat_server.verify(key.clone(),conn).await {
                Ok(server_name) => {
                    log::info!("{}已上线",server_name);
                    let template = json!({
                    "code": 1,
                    "server_name": server_name
                });
                    match session.text(template.to_string()).await{
                        Ok(_) => {true}
                        Err(e) => {
                            log::error!("向客户端发送响应失败:{}",e);
                            false
                        }
                    }
                },
                Err(e) if e.is::<DuplicateConnectionsError>() => {
                    log::error!("密钥{}当前已有在线的客户端",key);
                    let template = json!({
                        "code": -2,
                    });
                    match session.text(template.to_string()).await{
                        Ok(_) => {false}
                        Err(e) => {
                            log::error!("向客户端发送响应失败:{}",e);
                            false
                        }
                    }
                }
                Err(e) if e.is::<NoSuchKeyError>() => {
                    log::error!("客户端密钥{}无效",key);
                    let template = json!({
                        "code": -1,
                    });
                    match session.text(template.to_string()).await{
                        Ok(_) => {false}
                        Err(e) => {
                            log::error!("向客户端发送响应失败:{}",e);
                            false
                        }
                    }
                },

                _ => {false}
            }
            /*
            match (code, key) {
                (Some(code_val), Some(key_val)) => {
                    println!("Code: {}", code_val);
                    println!("Key: {}", key_val);
                }
                _ => println!("Failed to get values"),
            }
             */
        }
        Err(e) => {
            log::error!("客户端发送了无效的消息:{}",e);
            session.text("Invalid message").await.unwrap();
            false
        },
    }
    /*
    // unwrap: we have guaranteed non-zero string length already
    match cmd_args.next().unwrap() {
        "/list" => {
            log::info!("conn {conn}: listing rooms");

            let rooms = chat_server.list_rooms().await;

            for room in rooms {
                session.text(room).await.unwrap();
            }
        }

        "/join" => match cmd_args.next() {
            Some(room) => {
                log::info!("conn {conn}: joining room {room}");

                chat_server.join_room(conn, room).await;

                session.text(format!("joined {room}")).await.unwrap();
            }

            None => {
                session.text("!!! room name is required").await.unwrap();
            }
        },

        "/name" => match cmd_args.next() {
            Some(new_name) => {
                log::info!("conn {conn}: setting name to: {new_name}");
                name.replace(new_name.to_owned());
            }
            None => {
                session.text("!!! name is required").await.unwrap();
            }
        },

        _ => {
            session
                .text(format!("!!! unknown command: {msg}"))
                .await
                .unwrap();
        }
    }
     */
}