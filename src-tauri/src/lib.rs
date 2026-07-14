mod db;

use rusqlite::Connection;
use std::sync::Mutex;
use tauri::State;

struct AppState {
    db: Mutex<Connection>,
}

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
    Ok(db::create_connection(&conn, &name, &host, port, &user, &pass))
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

#[tauri::command]
fn get_session_logs(state: State<AppState>) -> Result<Vec<db::SessionLog>, String> {
    let conn = state.db.lock().unwrap();
    Ok(db::list_session_logs(&conn))
}

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
        })
        .invoke_handler(tauri::generate_handler![
            get_connections,
            add_connection,
            update_connection,
            remove_connection,
            get_session_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
