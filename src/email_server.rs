use crate::error::NoSuchValueError;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use crate::CONFIG;
use std::collections::HashMap;
use std::sync::Arc;
use std::{error::Error, io};
use tokio::sync::RwLock;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{self, Duration};
use uuid::Uuid;

#[derive(Debug)]
enum Command {
    SendToken {
        email: String,
        server_name: String,
        res_tx: oneshot::Sender<Result<(), Box<dyn Error + Send + Sync>>>,
    },
    ValidateToken {
        token: String,
        res_tx: oneshot::Sender<Result<String, Box<dyn Error + Send + Sync>>>,
    },
}

pub struct EmailServer {
    cmd_rx: mpsc::UnboundedReceiver<Command>,
    /// 邮件地址和(token,过期时间)的HashMap
    tokens: Arc<RwLock<HashMap<String, (String, time::Instant)>>>,
    /// token和服务器名的HashMap
    server_names: Arc<RwLock<HashMap<String, String>>>,
    smtp_transport: AsyncSmtpTransport<Tokio1Executor>,
}

impl EmailServer {
    pub fn new() -> (EmailServer, EmailServerHandle) {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();

        let creds = Credentials::new("notify@toho.red".to_string(), "of2ghuE.".to_string());
        let smtp_transport = AsyncSmtpTransport::<Tokio1Executor>::relay("smtp.zoho.com")
            .unwrap()
            .credentials(creds)
            .build();

        (
            EmailServer {
                cmd_rx,
                tokens: Arc::new(RwLock::new(HashMap::new())),
                server_names: Arc::new(RwLock::new(HashMap::new())),
                smtp_transport,
            },
            EmailServerHandle {
                cmd_tx,
            },
        )
    }

    pub async fn send_token(&mut self, email: String, server_name: String) -> Result<(), Box<dyn Error + Send + Sync>> {
        let token = Uuid::new_v4().to_string();
        let link = format!("https://awl.toho.red/verify/{}",token);

        let message = Message::builder()
            .from("notify@toho.red".parse()?)
            .to(email.parse()?)
            .subject("autowhitelist验证邮件")
            .body(format!(
                "尊敬的用户您好，欢迎注册autowhitelist服务，您的验证链接为: {} ，只需点击即可完成注册。\n\
                如果您没有注册过相关服务，请忽略本邮件，祝您生活愉快。
            ", link))?;

        self.smtp_transport.send(message).await.map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;

        // 写入HashMap
        self.tokens.write().await.insert(token.clone(), (email, time::Instant::now()));
        self.server_names.write().await.insert(token, server_name);
        Ok(())
    }

    pub async fn validate_token(&self, token: String) -> Result<String, Box<dyn Error + Send + Sync>> {
        let mut tokens = self.tokens.write().await;
        let mut server_names = self.server_names.write().await;
        println!("{:?}",server_names);
        // 如果token存在则返回对应服务器名
        if let Some((_email, _)) = tokens.remove(&token) {
            return Ok(server_names.remove(&token).unwrap());
        }
        Err(Box::new(NoSuchValueError))
    }

    pub async fn run(mut self) -> io::Result<()> {
        let mut interval = time::interval(Duration::from_secs(60));
        let tokens = self.tokens.clone();
        let server_names = self.server_names.clone();
        loop {
            tokio::select! {
                // 处理命令
                Some(cmd) = self.cmd_rx.recv() => {
                    match cmd {
                        Command::SendToken { email, server_name, res_tx } => {
                            let result = self.send_token(email, server_name).await;
                            let _ = res_tx.send(result);
                        }
                        Command::ValidateToken { token, res_tx } => {
                            let result = self.validate_token(token).await;
                            let _ = res_tx.send(result);
                        }
                    }
                }
                // 定时清除过期token
                _ = interval.tick() => {
                    let now = time::Instant::now();
                    let tokens = tokens.write().await;
                    let mut server_names = server_names.write().await;
                    // 移除过期的token以及对应的服务器名
                    for (token, (_, expire_time)) in tokens.iter() {
                        if now > *expire_time {
                            server_names.remove(token);
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct EmailServerHandle {
    cmd_tx: mpsc::UnboundedSender<Command>,
}

impl EmailServerHandle {
    /// 生成一个新的携带token的链接并发送至指定邮箱，同时存储服务器名
    pub async fn send_token(&self, email: String, server_name: String) -> Result<(), Box<dyn Error + Send + Sync>> {
        let (res_tx, res_rx) = oneshot::channel();
        self.cmd_tx
            .send(Command::SendToken { email, server_name, res_tx })
            .unwrap();

        res_rx.await.unwrap()
    }

    /// 当用户点击url时验证token合法性，如果合法则返回服务器名
    pub async fn validate_token(&self, token: String) -> Result<String, Box<dyn Error + Send + Sync>> {
        let (res_tx, res_rx) = oneshot::channel();
        self.cmd_tx
            .send(Command::ValidateToken { token, res_tx })
            .unwrap();

        res_rx.await.unwrap()
    }
}
