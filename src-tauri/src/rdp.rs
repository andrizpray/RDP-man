use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::{Child, Command};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RdpConfig {
    pub connection_id: i64,
    pub hostname: String,
    pub port: i32,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionInfo {
    pub session_id: u32,
    pub connection_id: i64,
    pub hostname: String,
    pub pid: u32,
    pub status: String,
}

pub struct SessionManager {
    sessions: HashMap<u32, (Child, RdpConfig)>,
    next_id: u32,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn open(&mut self, config: RdpConfig) -> Result<SessionInfo, String> {
        let addr = format!("{}:{}", config.hostname, config.port);

        // Detect platform and use appropriate RDP client
        let (cmd, args) = if cfg!(target_os = "windows") {
            (
                "mstsc.exe",
                vec![format!("/v:{}", addr), format!("/u:{}", config.username)],
            )
        } else if cfg!(target_os = "macos") {
            // macOS: open .rdp file or use Microsoft Remote Desktop
            // Fallback: try xfreerdp via Homebrew
            (
                "xfreerdp",
                vec![
                    format!("/v:{}", addr),
                    format!("/u:{}", config.username),
                    format!("/p:{}", config.password),
                    "/cert:ignore".to_string(),
                    "/dynamic-resolution".to_string(),
                    "/clipboard".to_string(),
                    "/audio-mode:0".to_string(),
                ],
            )
        } else {
            // Linux: FreeRDP CLI
            (
                "xfreerdp3",
                vec![
                    format!("/v:{}", addr),
                    format!("/u:{}", config.username),
                    format!("/p:{}", config.password),
                    "/cert:ignore".to_string(),
                    "/dynamic-resolution".to_string(),
                    "/clipboard".to_string(),
                    "/audio-mode:0".to_string(),
                    "/drive:home,/home".to_string(),
                ],
            )
        };

        let child = Command::new(cmd).args(&args).spawn().map_err(|e| {
            format!(
                "Failed to launch RDP client: {}. Is xfreerdp3/xfreerdp installed?",
                e
            )
        })?;

        let pid = child.id();
        let session_id = self.next_id;
        self.next_id += 1;
        let hostname = config.hostname.clone();

        let info = SessionInfo {
            session_id,
            connection_id: config.connection_id,
            hostname,
            pid,
            status: "connected".to_string(),
        };

        self.sessions.insert(session_id, (child, config));
        Ok(info)
    }

    pub fn close(&mut self, session_id: u32) -> Result<(), String> {
        if let Some((mut child, _)) = self.sessions.remove(&session_id) {
            child.kill().map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err("Session not found".to_string())
        }
    }

    pub fn list_active(&mut self) -> Vec<SessionInfo> {
        // Clean up dead processes
        let mut dead = vec![];
        for (id, (child, _)) in &mut self.sessions {
            match child.try_wait() {
                Ok(Some(_)) => dead.push(*id),
                _ => {}
            }
        }
        for id in dead {
            self.sessions.remove(&id);
        }

        self.sessions
            .iter()
            .map(|(id, (child, config))| SessionInfo {
                session_id: *id,
                connection_id: config.connection_id,
                hostname: config.hostname.clone(),
                pid: child.id(),
                status: "connected".to_string(),
            })
            .collect()
    }

    pub fn close_all(&mut self) {
        for (_, (mut child, _)) in self.sessions.drain() {
            let _ = child.kill();
        }
    }
}

// ponytail: global mutex, fine for <20 sessions. Per-session locks if perf matters.
pub fn new_manager() -> Mutex<SessionManager> {
    Mutex::new(SessionManager::new())
}
