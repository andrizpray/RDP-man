mod db;
mod health;
mod rdp;

use rusqlite::Connection;
use std::sync::Mutex;
use tauri::State;

struct AppState {
    db: Mutex<Connection>,
    rdp: Mutex<rdp::SessionManager>,
}

// ── Connection CRUD ──

#[tauri::command]
fn get_connections(state: State<AppState>) -> Result<Vec<db::ConnectionProfile>, String> {
    let conn = state.db.lock().unwrap();
    Ok(db::list_connections(&conn))
}

#[tauri::command]
fn add_connection(
    state: State<AppState>,
    name: String,
    host: String,
    port: i32,
    user: String,
    pass: String,
) -> Result<i64, String> {
    let conn = state.db.lock().unwrap();
    Ok(db::create_connection(
        &conn, &name, &host, port, &user, &pass,
    ))
}

#[tauri::command]
fn update_connection(
    state: State<AppState>,
    id: i64,
    name: String,
    host: String,
    port: i32,
    user: String,
    pass: String,
) -> Result<(), String> {
    let conn = state.db.lock().unwrap();
    db::update_connection(&conn, id, &name, &host, port, &user, &pass);
    Ok(())
}

#[tauri::command]
fn remove_connection(state: State<AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().unwrap();
    db::delete_connection(&conn, id);
    Ok(())
}

// ── Health Check ──

#[tauri::command]
fn check_server(host: String, port: u16) -> health::ServerStatus {
    health::probe(&host, port)
}

#[tauri::command]
fn check_all_servers(state: State<AppState>) -> Result<Vec<health::HealthResult>, String> {
    let conn = state.db.lock().unwrap();
    let connections = db::list_connections(&conn);
    Ok(connections
        .iter()
        .map(|c| health::HealthResult {
            id: c.id,
            hostname: c.hostname.clone(),
            port: c.port,
            status: health::probe(&c.hostname, c.port as u16),
        })
        .collect())
}

// ── RDP Sessions ──

#[tauri::command]
fn open_rdp_session(
    state: State<AppState>,
    connection_id: i64,
) -> Result<rdp::SessionInfo, String> {
    let conn = state.db.lock().unwrap();
    let connections = db::list_connections(&conn);
    let profile = connections
        .iter()
        .find(|c| c.id == connection_id)
        .ok_or("Connection not found")?;

    let config = rdp::RdpConfig {
        connection_id: profile.id,
        hostname: profile.hostname.clone(),
        port: profile.port,
        username: profile.username.clone(),
        password: profile.password.clone(),
    };

    let log_id = db::log_session_start(&conn, profile.id, &profile.hostname, &profile.username);
    drop(conn); // release DB lock before spawning process

    let mut mgr = state.rdp.lock().unwrap();
    let info = mgr.open(config)?;

    // Log will be finalized when session closes (Phase 4+)
    // ponytail: just log start for now, end on explicit close
    let _ = log_id;

    Ok(info)
}

#[tauri::command]
fn close_rdp_session(state: State<AppState>, session_id: u32) -> Result<(), String> {
    let mut mgr = state.rdp.lock().unwrap();
    mgr.close(session_id)
}

#[tauri::command]
fn get_active_sessions(state: State<AppState>) -> Vec<rdp::SessionInfo> {
    let mut mgr = state.rdp.lock().unwrap();
    mgr.list_active()
}

// ── Session History ──

#[tauri::command]
fn get_session_logs(state: State<AppState>) -> Result<Vec<db::SessionLog>, String> {
    let conn = state.db.lock().unwrap();
    Ok(db::list_session_logs(&conn))
}

// ── App Entry ──

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db_path = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("rdp-man");
    std::fs::create_dir_all(&db_path).unwrap();
    let db_conn = Connection::open(db_path.join("rdp-man.db")).unwrap();
    db::init_db(&db_conn);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            db: Mutex::new(db_conn),
            rdp: rdp::new_manager(),
        })
        .invoke_handler(tauri::generate_handler![
            get_connections,
            add_connection,
            update_connection,
            remove_connection,
            check_server,
            check_all_servers,
            open_rdp_session,
            close_rdp_session,
            get_active_sessions,
            get_session_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
