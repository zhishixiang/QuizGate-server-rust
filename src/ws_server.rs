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
use crate::{ConnId, Key, PlayerId};
use rand::{thread_rng, Rng as _, random};
use sqlx::Sqlite;
use sqlx_core::Error;
use sqlx_core::pool::Pool;

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
        key: Key,
        res_tx: oneshot::Sender<()>,
    },

    Verify {
        key:Key,
        res_tx: oneshot::Sender<(String)>
    }
}

#[derive(Debug)]
pub struct WsServer {
    /// 链接ID和消息发送管道的键值对
    sessions: HashMap<ConnId, mpsc::UnboundedSender<PlayerId>>,

    /// 客户端key和链接id的键值对
    client_list: HashMap<Key,ConnId>,

    /// 维护的链接总数
    visitor_count: Arc<AtomicUsize>,

    /// 接收命令的管道
    cmd_rx: mpsc::UnboundedReceiver<Command>,

    /// sql命令池
    sql_pool: Arc<Pool<Sqlite>>
}

impl WsServer {
    pub fn new(sql_pool:Arc<Pool<Sqlite>>) -> (WsServer, WsServerHandle) {

        let (cmd_tx,cmd_rx) = mpsc::unbounded_channel();
        (
            WsServer{
                sessions: HashMap::new(),
                client_list: HashMap::new(),
                visitor_count: Arc::new(AtomicUsize::new(0)),
                cmd_rx,
                sql_pool
            },
            WsServerHandle {
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
    async fn disconnect(&mut self, conn_id: ConnId) {
        // 从表中移除链接
        self.sessions.remove(&conn_id);
    }
    async fn verify(&self, key: Key) -> Result<String,Error>{
        let result: Result<Option<(String, )>, Error> = sqlx::query_as("SELECT name FROM server_info WHERE key = ?")
            .bind(&key)
            .fetch_optional(&*self.sql_pool)
            .await;
        match result{
            Ok(Some(row)) => {
                println!("客户端{}上线", row.0);
                Ok(row.0)
        }
            Ok(None) => {
                eprintln!("客户端发送了无效的key，断开链接");
                Err(Error::RowNotFound)
            }
            Err(e) => {
                log::error!("查询客户端密钥失败:{}",e);
                Err(e)
            }
        }
    }
    async fn add_player(&self, key: Key, player_id: PlayerId){
        let conn_id = self.client_list[&key];
        for session in self.sessions.clone() {
            if conn_id.eq(&session.0) {
                    session.1.send(player_id.clone()).unwrap();
                }
            }
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


                Command::AddPlayer { key, id, res_tx} => {
                    self.add_player(key, id).await;
                    let _ = res_tx.send(());
                }

                Command::Verify { key, res_tx} => {
                    let res = self.verify(key).await.unwrap();
                    let _ = res_tx.send(res);
                }
            }
        }

        Ok(())
    }

}
#[derive(Debug, Clone)]
pub struct WsServerHandle {
    cmd_tx: mpsc::UnboundedSender<Command>,
}

impl WsServerHandle {
    /// 处理来自客户端的连接
    pub async fn connect(&self, conn_tx: mpsc::UnboundedSender<Key>) -> Result<ConnId, io::Error> {
        let (res_tx, res_rx) = oneshot::channel();
        let id = random::<ConnId>();
        self.cmd_tx
            .send(Command::Connect { conn_tx, res_tx })
            .unwrap();

        // unwrap: chat server does not drop out response channel
        res_rx.await.unwrap();
        Ok(id)
    }

    /// 验证客户端密钥
    pub async fn verify(&self,key: Key) -> Result<ConnId, io::Error> {
        let (res_tx, res_rx) = oneshot::channel();
        self.cmd_tx
            .send(Command::Verify {key, res_tx })
            .unwrap();

        // unwrap: chat server does not drop out response channel
        let res = res_rx.await.unwrap();
        Ok(res.parse().unwrap())
    }

    /// 向特定客户端发送消息
    pub async fn send_message(&self, key: Key, player_id: impl Into<PlayerId>) {
        let (res_tx, res_rx) = oneshot::channel();

        // 将指令发送到指定的客户端
        self.cmd_tx
            .send(Command::AddPlayer {
                id: player_id.into(),
                key,
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