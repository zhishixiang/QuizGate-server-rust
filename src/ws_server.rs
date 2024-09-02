use std::{
    collections::{HashMap, HashSet},
    io,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use std::ops::Deref;
use tokio::sync::{mpsc, oneshot};
use crate::exam_webserver::{ConnId, Key, PlayerId};
use rand::{thread_rng, Rng as _, random};

#[derive(Debug)]
enum Command {
    Connect {
        conn_tx: mpsc::UnboundedSender<Key>,
        res_tx: oneshot::Sender<ConnId>,
    },

    Disconnect {
        conn: ConnId,
    },

    AddPlayer {
        id: PlayerId,
        conn: ConnId,
        res_tx: oneshot::Sender<()>,
    },

}

#[derive(Debug)]
pub struct WsServer {
    /// 链接ID和接收者的键值对
    sessions: HashMap<ConnId, mpsc::UnboundedSender<PlayerId>>,


    /// 维护的链接总数
    visitor_count: Arc<AtomicUsize>,

    /// 接收命令的管道
    cmd_rx: mpsc::UnboundedReceiver<Command>,
}

impl WsServer {
    async fn new() -> (Self, ChatServerHandle){
        let (cmd_tx,cmd_rx) = mpsc::unbounded_channel();
        (
            WsServer{
                sessions: HashMap::new(),
                visitor_count: Arc::new(AtomicUsize::new(0)),
                cmd_rx
            },
            ChatServerHandle{
                cmd_tx,
            }
        )
    }
    async fn connect(&mut self, tx: mpsc::UnboundedSender<PlayerId>) -> ConnId{
        // 生成id并插入表
        let id = random::<ConnId>();
        self.sessions.insert(id,tx);
        // 计数器+1
        self.visitor_count.fetch_add(1, Ordering::SeqCst);
        id
    }
    pub async fn run(mut self) -> io::Result<()> {
        while let Some(cmd) = self.cmd_rx.recv().await {
            match cmd {
                Command::Connect { conn_tx, res_tx } => {
                    let conn_id = self.connect(conn_tx).await;
                    let _ = res_tx.send(conn_id);
                }

                Command::Disconnect { conn } => {
                    self.disconnect(conn).await;
                }


                Command::AddPlayer { conn, id, res_tx} => {
                    self.send_message(conn, id).await;
                    let _ = res_tx.send(());
                }
            }
        }

        Ok(())
    }
}
#[derive(Debug, Clone)]
pub struct ChatServerHandle {
    cmd_tx: mpsc::UnboundedSender<Command>,
}

impl ChatServerHandle {
    /// 处理来自客户端的连接
    pub async fn connect(&self, conn_tx: mpsc::UnboundedSender<Key>) -> ConnId {
        let (res_tx, res_rx) = oneshot::channel();

        // 验证客户端密钥
        todo!();

        // 向服务器注册客户端
        self.cmd_tx
            .send(Command::Connect { conn_tx, res_tx })
            .unwrap();

        // unwrap: chat server does not drop out response channel
        res_rx.await.unwrap()
    }


    /// 向特定客户端发送消息
    pub async fn send_message(&self, conn: ConnId, player_id: impl Into<PlayerId>) {
        let (res_tx, res_rx) = oneshot::channel();

        // 将指令发送到指定的客户端
        self.cmd_tx
            .send(Command::AddPlayer {
                id: player_id,
                conn,
                res_tx,
            })
            .unwrap();

        // unwrap: chat server does not drop our response channel
        res_rx.await.unwrap();
    }

    /// 断开链接并从服务器注销链接
    pub fn disconnect(&self, conn: ConnId) {
        // unwrap: chat server should not have been dropped
        self.cmd_tx.send(Command::Disconnect { conn }).unwrap();
    }
}