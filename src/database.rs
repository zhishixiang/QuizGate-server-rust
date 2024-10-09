use log::error;
use tokio::sync::{mpsc, oneshot};
use sqlx_core::Error;
use sqlx::{pool::Pool, sqlite::{Sqlite, SqlitePoolOptions}};

use crate::structs::awl_type::{SqlFile, SqlStatement};


#[derive(Debug)]
enum Command {
    Connect {
        conn_tx: mpsc::UnboundedSender<SqlFile>,
        res_tx: oneshot::Sender<bool>,
    },
    Execute {
        conn_tx: mpsc::UnboundedSender<SqlStatement>,
        res_tx: oneshot::Sender<bool>,
    }
}

pub struct SqlServer{
    // sql连接池
    pool: Pool<Sqlite>,
    
    /// 接收命令的管道
    cmd_rx: mpsc::UnboundedReceiver<Command>,

    
}
impl SqlServer{
    pub async fn new_sql_pool(sql_file:SqlFile) -> bool {
        // 创建一个连接池
        let pool = match SqlitePoolOptions::new()
            .max_connections(5)
            .connect(format!("sqlite://{}",sql_file).as_str())
            .await{
                Ok(pool) => pool,
                Err(e) => {
                    error!("创建SQL连接池失败: {:?}", e);
                    return false;
                }
            };
    
        // 执行创建表的 SQL 语句
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS server_info (
                id    INTEGER PRIMARY KEY AUTOINCREMENT,
                name  TEXT NOT NULL,
                key   TEXT NOT NULL
            )"
        )
            .execute(&pool)
            .await.expect("执行创建表命令失败！");
        true
    }
    
}
