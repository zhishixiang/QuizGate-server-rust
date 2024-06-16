use rusqlite::{Connection, Result};
pub async fn new_database_conn() -> Result<(Connection)>{
    let conn = Connection::open("data.db")?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS server_info (
            id    INTEGER PRIMARY KEY AUTOINCREMENT,
            name  TEXT NOT NULL,
            key  TEXT NOT NULL
        )",
        (), // empty list of parameters.
    )?;
    Ok(conn)
}
