use futures_util::{StreamExt, SinkExt};
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, tungstenite::protocol::Message, WebSocketStream};
use tokio::net::TcpStream;

type WsStream = WebSocketStream<TcpStream>;

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:9001";
    let listener = TcpListener::bind(&addr).await.unwrap();
    println!("Listening on: {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(async move {
            match accept_async(stream).await {
                Ok(ws_stream) => {
                    let (mut write, mut read) = ws_stream.split();
                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(msg) => {
                                println!("{}", msg.clone().into_text().unwrap());
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
                Err(e) => {
                    eprintln!("Error during WebSocket handshake: {}", e);
                }
            }
        });
    }
}
