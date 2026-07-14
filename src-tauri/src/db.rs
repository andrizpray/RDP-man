use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionProfile {
    pub id: i64,
    pub display_name: String,
    pub hostname: String,
    pub port: i32,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionLog {
    pub id: i64,
    pub connection_id: i64,
    pub hostname: String,
    pub username: String,
    pub connected_at: String,
    pub disconnected_at: Option<String>,
    pub duration_sec: Option<i64>,
    pub status: String,
}

pub fn init_db(conn: &Connection) {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS connections (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            display_name TEXT NOT NULL,
            hostname TEXT NOT NULL,
            port INTEGER NOT NULL DEFAULT 3389,
            username TEXT NOT NULL,
            password TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE TABLE IF NOT EXISTS sessions_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            connection_id INTEGER NOT NULL REFERENCES connections(id),
            hostname TEXT NOT NULL,
            username TEXT NOT NULL,
            connected_at TEXT NOT NULL,
            disconnected_at TEXT,
            duration_sec INTEGER,
            status TEXT NOT NULL CHECK(status IN ('connected','disconnected','error','reconnecting'))
        );"
    ).unwrap();
}

pub fn create_connection(
    conn: &Connection,
    name: &str,
    host: &str,
    port: i32,
    user: &str,
    pass: &str,
) -> i64 {
    conn.execute(
        "INSERT INTO connections (display_name, hostname, port, username, password) VALUES (?1,?2,?3,?4,?5)",
        params![name, host, port, user, pass],
    ).unwrap();
    conn.last_insert_rowid()
}

pub fn list_connections(conn: &Connection) -> Vec<ConnectionProfile> {
    conn.prepare(
        "SELECT id, display_name, hostname, port, username, password FROM connections ORDER BY id",
    )
    .unwrap()
    .query_map([], |row| {
        Ok(ConnectionProfile {
            id: row.get(0)?,
            display_name: row.get(1)?,
            hostname: row.get(2)?,
            port: row.get(3)?,
            username: row.get(4)?,
            password: row.get(5)?,
        })
    })
    .unwrap()
    .filter_map(|r| r.ok())
    .collect()
}

pub fn update_connection(
    conn: &Connection,
    id: i64,
    name: &str,
    host: &str,
    port: i32,
    user: &str,
    pass: &str,
) {
    conn.execute(
        "UPDATE connections SET display_name=?1, hostname=?2, port=?3, username=?4, password=?5, updated_at=datetime('now') WHERE id=?6",
        params![name, host, port, user, pass, id],
    ).unwrap();
}

pub fn delete_connection(conn: &Connection, id: i64) {
    conn.execute("DELETE FROM connections WHERE id=?1", params![id])
        .unwrap();
}

#[allow(dead_code)]
pub fn log_session_start(conn: &Connection, conn_id: i64, host: &str, user: &str) -> i64 {
    conn.execute(
        "INSERT INTO sessions_log (connection_id, hostname, username, connected_at, status) VALUES (?1,?2,?3,datetime('now'),'connected')",
        params![conn_id, host, user],
    ).unwrap();
    conn.last_insert_rowid()
}

#[allow(dead_code)]
pub fn log_session_end(conn: &Connection, log_id: i64, status: &str) {
    conn.execute(
        "UPDATE sessions_log SET disconnected_at=datetime('now'), status=?1, \
         duration_sec=CAST((julianday(datetime('now')) - julianday(connected_at)) * 86400 AS INTEGER) \
         WHERE id=?2",
        params![status, log_id],
    ).unwrap();
}

pub fn list_session_logs(conn: &Connection) -> Vec<SessionLog> {
    conn.prepare(
        "SELECT id, connection_id, hostname, username, connected_at, disconnected_at, duration_sec, status \
         FROM sessions_log ORDER BY id DESC LIMIT 500"
    )
    .unwrap()
    .query_map([], |row| {
        Ok(SessionLog {
            id: row.get(0)?,
            connection_id: row.get(1)?,
            hostname: row.get(2)?,
            username: row.get(3)?,
            connected_at: row.get(4)?,
            disconnected_at: row.get(5)?,
            duration_sec: row.get(6)?,
            status: row.get(7)?,
        })
    })
    .unwrap()
    .filter_map(|r| r.ok())
    .collect()
}
