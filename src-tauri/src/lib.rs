mod db;
mod health;
mod ironrdp_session;
mod rdp;

use rusqlite::Connection;
use std::sync::Mutex;
use tauri::State;

struct AppState {
    db: Mutex<Connection>,
    rdp: Mutex<rdp::SessionManager>,
    ironrdp: Mutex<ironrdp_session::SessionManager>,
}

// ── Connection CRUD ──

#[tauri::command]
fn get_connections(state: State<AppState>) -> Result<Vec<db::ConnectionProfile>, String> {
    Ok(db::list_connections(&state.db.lock().unwrap()))
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
    Ok(db::create_connection(
        &state.db.lock().unwrap(),
        &name,
        &host,
        port,
        &user,
        &pass,
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
    db::update_connection(
        &state.db.lock().unwrap(),
        id,
        &name,
        &host,
        port,
        &user,
        &pass,
    );
    Ok(())
}

#[tauri::command]
fn remove_connection(state: State<AppState>, id: i64) -> Result<(), String> {
    db::delete_connection(&state.db.lock().unwrap(), id);
    Ok(())
}

// ── Health ──

#[tauri::command]
fn check_all_servers(state: State<AppState>) -> Result<Vec<health::HealthResult>, String> {
    let conns = db::list_connections(&state.db.lock().unwrap());
    Ok(conns
        .iter()
        .map(|c| health::HealthResult {
            id: c.id,
            hostname: c.hostname.clone(),
            port: c.port,
            status: health::probe(&c.hostname, c.port as u16),
        })
        .collect())
}

// ── RDP Sessions (in-app via IronRDP) ──

#[tauri::command]
fn open_rdp_session(
    state: State<AppState>,
    connection_id: i64,
) -> Result<ironrdp_session::SessionInfo, String> {
    let conns = db::list_connections(&state.db.lock().unwrap());
    let profile = conns
        .iter()
        .find(|c| c.id == connection_id)
        .ok_or("Connection not found")?;

    let config = ironrdp_session::RdpConfig {
        connection_id: profile.id,
        hostname: profile.hostname.clone(),
        port: profile.port as u16,
        username: profile.username.clone(),
        password: profile.password.clone(),
        width: 1920,
        height: 1080,
    };

    state.ironrdp.lock().unwrap().open(config)
}

#[tauri::command]
fn close_rdp_session(state: State<AppState>, session_id: u32) -> Result<(), String> {
    state.ironrdp.lock().unwrap().close(session_id)
}

#[tauri::command]
fn get_active_sessions(state: State<AppState>) -> Vec<ironrdp_session::SessionInfo> {
    state.ironrdp.lock().unwrap().list_active()
}

#[tauri::command]
fn get_framebuffer(state: State<AppState>, session_id: u32) -> Result<Vec<u32>, String> {
    state
        .ironrdp
        .lock()
        .unwrap()
        .get_framebuffer(session_id)
        .ok_or("Session not found".to_string())
}

#[tauri::command]
fn send_rdp_input(
    state: State<AppState>,
    session_id: u32,
    event: ironrdp_session::InputEvent,
) -> Result<(), String> {
    state.ironrdp.lock().unwrap().send_input(session_id, event)
}

// ── Session History ──

#[tauri::command]
fn get_session_logs(state: State<AppState>) -> Result<Vec<db::SessionLog>, String> {
    Ok(db::list_session_logs(&state.db.lock().unwrap()))
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
            ironrdp: ironrdp_session::new_manager(),
        })
        .invoke_handler(tauri::generate_handler![
            get_connections,
            add_connection,
            update_connection,
            remove_connection,
            check_all_servers,
            open_rdp_session,
            close_rdp_session,
            get_active_sessions,
            get_framebuffer,
            send_rdp_input,
            get_session_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
