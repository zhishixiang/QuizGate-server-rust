use log;
use tokio::time::{self, Duration};
use tokio::sync::{mpsc, oneshot};
use sqlx::{pool::Pool, sqlite::{Sqlite, SqlitePoolOptions}};
use std::{error::Error, io};
use std::path::Path;
use crate::{error::NoSuchValueError, r#struct::awl_type::SqlFile};

#[derive(Debug)]
enum Command {
    Execute {
        sql_statement: SqlStatement,
        res_tx: oneshot::Sender<Result<String, Box<dyn Error + Send + Sync>>>,
    }
}

pub struct SqlServer {
    // sql连接池
    pool: Pool<Sqlite>,
    
    /// 接收命令的管道
    cmd_rx: mpsc::UnboundedReceiver<Command>,
}

#[derive(Debug, Clone)]
pub struct SqlStatement {
    pub sql: String,
    pub params: Vec<String>,
}

impl SqlStatement {
    pub fn as_str(&self) -> &str {
        &self.sql
    }

    pub fn params(&self) -> &[String] {
        &self.params
    }
}

impl SqlServer {
    pub async fn new(sql_file: SqlFile) -> Result<(SqlServer, SqlServerHandle), Box<dyn Error>> {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        // 检测数据库文件是否存在，不存在则新建
        if !Path::new(sql_file.as_str()).exists() {
            log::info!("数据库文件不存在，创建数据库文件: {}", sql_file.as_str());
            let file = std::fs::File::create(sql_file.as_str()).map_err(|e| {
                log::error!("创建数据库文件失败: {:?}", e);
                Box::new(e) as Box<dyn Error>
            })?;
            file.sync_all().map_err(|e| {
                log::error!("同步数据库文件失败: {:?}", e);
                Box::new(e) as Box<dyn Error>
            })?;
        }
        // 创建一个连接池
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(format!("sqlite://{}", sql_file).as_str())
            .await
            .map_err(|e| {
                log::error!("创建SQL连接池失败: {:?}", e);
                Box::new(e) as Box<dyn Error>
            })?;
    
        // 执行创建表的 SQL 语句
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS server_info (
                id    INTEGER PRIMARY KEY AUTOINCREMENT,
                name  TEXT NOT NULL,
                key   TEXT NOT NULL
            )"
        )
        .execute(&pool)
        .await
        .map_err(|e| {
            log::error!("执行创建表命令失败: {:?}", e);
            Box::new(e) as Box<dyn Error>
        })?;
    
        Ok((
            SqlServer {
                pool,
                cmd_rx,
            },
            SqlServerHandle {
                cmd_tx,
            },
        ))
    }

    pub async fn execute_statement(&mut self, sql_statement: SqlStatement) -> Result<String, Box<dyn Error + Send + Sync>> {
        let mut query = sqlx::query_as::<_, (String,)>(sql_statement.as_str());
        // 先绑定参数后查询
        for param in sql_statement.params() {
            query = query.bind(param);
        }
        let result: Result<Option<(String,)>, sqlx::Error> = query.fetch_optional(&self.pool).await;
        match result {
            Ok(Some(row)) => Ok(row.0),
            Ok(None) => Err(Box::new(NoSuchValueError)),
            Err(e) => Err(Box::new(e)),
        }
}

    pub async fn run(mut self) -> io::Result<()> {
        let mut interval = time::interval(Duration::from_secs(5));

        loop {
            tokio::select! {
                Some(cmd) = self.cmd_rx.recv() => {
                    match cmd {
                        Command::Execute { sql_statement, res_tx } => {
                            let result = self.execute_statement(sql_statement).await;
                            let _ = res_tx.send(result);
                        }
                    }
                }
                _ = interval.tick() => {
                    // 这里可以添加定期执行的任务
                }
            }
        }
    }
}
#[derive(Debug, Clone)]
pub struct SqlServerHandle {
    cmd_tx: mpsc::UnboundedSender<Command>,
}
impl SqlServerHandle {
    pub async fn execute(&self,sql_statement:SqlStatement) -> Result<String, Box<dyn Error + Send + Sync>>{
        let (res_tx, res_rx) = oneshot::channel();
        self.cmd_tx
            .send(Command::Execute { sql_statement, res_tx })
            .unwrap();

        // unwrap: chat server does not drop out response channel
        res_rx.await.unwrap()
    }
}