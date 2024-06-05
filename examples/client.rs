use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;
use url::Url;
use futures_util::{SinkExt, StreamExt};

#[tokio::main]
async fn main() {
    // 连接到 WebSocket 服务器
    let url = Url::parse("ws://127.0.0.1:9001").unwrap();
    let (mut ws_stream, _) = connect_async(url).await.expect("Failed to connect");

    println!("WebSocket handshake has been successfully completed");

    // 发送消息到服务器
    let msg = Message::text("Hello, WebSocket server!");
    ws_stream.send(msg).await.expect("Failed to send message");

    // 接收服务器的响应
    if let Some(Ok(message)) = ws_stream.next().await {
        match message {
            Message::Text(text) => {
                println!("Received a text message from the server: {}", text);
            }
            Message::Binary(bin) => {
                println!("Received a binary message from the server: {:?}", bin);
            }
            _ => {
                println!("Received a different type of message from the server");
            }
        }
    }

    // 关闭 WebSocket 连接
    ws_stream.close(None).await.expect("Failed to close the connection");
}
