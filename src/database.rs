use sqlx_core::Error;
use sqlx::{pool::Pool, sqlite::{Sqlite, SqlitePoolOptions}};

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
