use tokio::sync::{mpsc, oneshot};
use sqlx_core::Error;
use sqlx::{pool::Pool, sqlite::{Sqlite, SqlitePoolOptions}};

use crate::error::CreateSqlPoolError;

#[derive(Debug)]
enum Command {
    Connect {
        conn_tx: mpsc::UnboundedSender<String>,
        res_tx: oneshot::Sender<CreateSqlPoolError>,
    },
    Execute {
        conn_tx: mpsc::UnboundedSender<Key>,
        res_tx: oneshot::Sender<ConnId>,
    }
}

pub struct SqlServer{
    // sql连接池
    pool: Pool<Sqlite>,
    
    /// 接收命令的管道
    cmd_rx: mpsc::UnboundedReceiver<Command>,

    
}
pub async fn new_sql_pool() -> Result<Pool<Sqlite>, Error> {
    // 创建一个连接池
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect("sqlite://data.db")
        .await?;

    // 执行创建表的 SQL 语句
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS server_info (
            id    INTEGER PRIMARY KEY AUTOINCREMENT,
            name  TEXT NOT NULL,
            key   TEXT NOT NULL
        )"
    )
        .execute(&pool)
        .await?;
    Ok(pool)
}
