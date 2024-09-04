use std::{
    pin::pin,
    time::{Duration, Instant},
};
use std::net::IpAddr;

use actix_ws::AggregatedMessage;
use futures_util::{
    future::{select, Either},
    StreamExt as _,
};
use log::log;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::{sync::mpsc, time::interval};
use tokio::sync::oneshot;
use warp::hyper::client::service::Connect;
use crate::ConnId;
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
    log::info!("connected");

    let mut name = None;
    let mut last_heartbeat = Instant::now();
    let mut interval = interval(HEARTBEAT_INTERVAL);

    let (conn_tx, mut conn_rx) = mpsc::unbounded_channel();

    // 验证客户端密钥

    let conn_id = match chat_server.connect(conn_tx).await{
        Ok(id) => id,
        Err(e) => {
            panic!("密钥错误，断开与客户端的链接");
        }
    };

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
                        process_text_msg(&chat_server, &mut session, &text, conn_id, &mut name)
                            .await;
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
                session.text(chat_msg).await.unwrap();
            }

            // all connection's message senders were dropped
            Either::Left((Either::Right((None, _)), _)) => unreachable!(
                "all connection message senders were dropped; chat server may have panicked"
            ),

            // heartbeat internal tick
            Either::Right((_inst, _)) => {
                // if no heartbeat ping/pong received recently, close the connection
                if Instant::now().duration_since(last_heartbeat) > CLIENT_TIMEOUT {
                    log::info!(
                        "client has not sent heartbeat in over {CLIENT_TIMEOUT:?}; disconnecting"
                    );
                    break None;
                }

                // send heartbeat ping
                let _ = session.ping(b"").await;
            }
        };
    };

    chat_server.disconnect(conn_id);

    // attempt to close connection gracefully
    let _ = session.close(close_reason).await;
}

async fn process_text_msg(
    chat_server: &WsServerHandle,
    session: &mut actix_ws::Session,
    text: &str,
    conn: ConnId,
    name: &mut Option<String>,
) {
    // 修剪掉多余换行符，虽然大概率不需要修剪
    let msg = text.trim();
    println!("{}", msg);
    let packet_recv: Result<serde_json::Value, serde_json::Error> = serde_json::from_str(text);
    match packet_recv {
        Ok(json) => {
            // 通过键来获取值
            let code = json["code"].as_i64();  // 获取 "code" 的值
            let key = json["key"].as_str();    // 获取 "key" 的值

            match (code, key) {
                (Some(code_val), Some(key_val)) => {
                    println!("Code: {}", code_val);
                    println!("Key: {}", key_val);
                }
                _ => println!("Failed to get values"),
            }
        }
        Err(e) => println!("Failed to parse JSON: {}", e),
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