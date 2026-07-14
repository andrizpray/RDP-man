use std::collections::HashMap;
use std::io;
use std::net::TcpStream;

use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;

use ironrdp_blocking::{connect_begin, connect_finalize, mark_as_upgraded, Framed};
use ironrdp_connector::{ClientConnector, Config, Credentials, DesktopSize, ServerName};
use ironrdp_core::WriteBuf;
use ironrdp_pdu::input::fast_path::FastPathInputEvent;

use ironrdp_graphics::image_processing::PixelFormat;
use ironrdp_pdu::input::mouse::PointerFlags;
use ironrdp_session::image::DecodedImage;
use ironrdp_session::{ActiveStageBuilder, ActiveStageOutput};

use ironrdp_connector::sspi::network_client::NetworkClient;
use ironrdp_connector::sspi::{Error as SspiError, NetworkRequest};
use ironrdp_pdu::rdp::client_info::PerformanceFlags;

use serde::{Deserialize, Serialize};

// ── Public types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RdpConfig {
    pub connection_id: i64,
    pub hostname: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionInfo {
    pub session_id: u32,
    pub connection_id: i64,
    pub hostname: String,
    pub status: String,
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputEvent {
    pub event_type: String, // "mouse_move", "mouse_click", "key_down", "key_up"
    pub x: u16,
    pub y: u16,
    pub button: u8, // 1=left, 2=right, 3=middle
    pub key_code: u16,
    pub is_down: bool,
}

// ── Shared framebuffer ──

pub type SharedBuffer = Arc<RwLock<Vec<u32>>>;

// ── Session manager ──

struct Session {
    info: SessionInfo,
    framebuffer: SharedBuffer,
    input_tx: std::sync::mpsc::Sender<InputEvent>,
    stop_flag: Arc<Mutex<bool>>,
    status: Arc<Mutex<String>>, // "connected" | "reconnecting" | "disconnected"
}

#[derive(Debug, Clone, Serialize)]
pub struct EndedSession {
    pub session_id: u32,
    pub connection_id: i64,
    pub hostname: String,
}

pub struct SessionManager {
    sessions: HashMap<u32, Session>,
    next_id: u32,
    ended_rx: std::sync::mpsc::Receiver<EndedSession>,
    ended_tx: std::sync::mpsc::Sender<EndedSession>,
}

impl SessionManager {
    pub fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            sessions: HashMap::new(),
            next_id: 1,
            ended_tx: tx,
            ended_rx: rx,
        }
    }

    /// Drain ended sessions for DB logging. Call from Tauri command.
    pub fn drain_ended(&self) -> Vec<EndedSession> {
        self.ended_rx.try_iter().collect()
    }

    pub fn open(&mut self, config: RdpConfig) -> Result<SessionInfo, String> {
        let session_id = self.next_id;
        self.next_id += 1;

        let w = config.width;
        let h = config.height;
        let framebuffer: SharedBuffer =
            Arc::new(RwLock::new(vec![0xFF000000u32; w as usize * h as usize]));
        let stop_flag = Arc::new(Mutex::new(false));
        let status = Arc::new(Mutex::new("connecting".to_string()));
        let (input_tx, input_rx) = std::sync::mpsc::channel::<InputEvent>();
        let input_rx = Arc::new(Mutex::new(input_rx));

        let fb = framebuffer.clone();
        let stop = stop_flag.clone();
        let st = status.clone();
        let hostname = config.hostname.clone();
        let conn_id = config.connection_id;

        let ended_tx = self.ended_tx.clone();
        let ended_info = EndedSession {
            session_id,
            connection_id: conn_id,
            hostname: hostname.clone(),
        };
        thread::spawn(move || {
            run_rdp_with_reconnect(config, fb, stop, input_rx, st);
            let _ = ended_tx.send(ended_info);
        });

        let info = SessionInfo {
            session_id,
            connection_id: conn_id,
            hostname,
            status: "connecting".to_string(),
            width: w,
            height: h,
        };

        self.sessions.insert(
            session_id,
            Session {
                info: info.clone(),
                framebuffer,
                input_tx,
                stop_flag,
                status,
            },
        );

        Ok(info)
    }

    pub fn close(&mut self, session_id: u32) -> Result<(), String> {
        if let Some(session) = self.sessions.remove(&session_id) {
            *session.stop_flag.lock().unwrap() = true;
            *session.status.lock().unwrap() = "disconnected".to_string();
            Ok(())
        } else {
            Err("Session not found".to_string())
        }
    }

    pub fn get_framebuffer(&self, session_id: u32) -> Option<Vec<u32>> {
        self.sessions
            .get(&session_id)?
            .framebuffer
            .read()
            .ok()
            .map(|b| b.clone())
    }

    pub fn send_input(&self, session_id: u32, event: InputEvent) -> Result<(), String> {
        self.sessions
            .get(&session_id)
            .ok_or("Session not found")?
            .input_tx
            .send(event)
            .map_err(|_| "Channel closed".to_string())
    }

    pub fn list_active(&self) -> Vec<SessionInfo> {
        self.sessions
            .values()
            .map(|s| {
                let mut info = s.info.clone();
                info.status = s.status.lock().unwrap().clone();
                info
            })
            .collect()
    }
}

// ── RDP session thread with auto-reconnect ──

fn run_rdp_with_reconnect(
    config: RdpConfig,
    framebuffer: SharedBuffer,
    stop_flag: Arc<Mutex<bool>>,
    input_rx: Arc<Mutex<std::sync::mpsc::Receiver<InputEvent>>>,
    status: Arc<Mutex<String>>,
) {
    let max_retries = 10u32;
    let mut retry = 0u32;
    let mut delay = Duration::from_secs(1);
    let max_delay = Duration::from_secs(30);

    loop {
        if *stop_flag.lock().unwrap() {
            break;
        }

        *status.lock().unwrap() = if retry == 0 {
            "connected".to_string()
        } else {
            format!("reconnecting ({}/{})", retry, max_retries)
        };

        // Clear framebuffer on reconnect
        if retry > 0 {
            if let Ok(mut fb) = framebuffer.write() {
                fb.fill(0xFF000000);
            }
        }

        match run_rdp_session(
            config.clone(),
            framebuffer.clone(),
            stop_flag.clone(),
            input_rx.clone(),
        ) {
            Ok(()) => {
                // Clean exit (server terminated or stop_flag set)
                if *stop_flag.lock().unwrap() {
                    break;
                }
                tracing::info!("RDP session ended, retrying...");
            }
            Err(e) => {
                tracing::warn!("RDP session error: {}", e);
            }
        }

        retry += 1;
        if retry > max_retries {
            tracing::error!("Max reconnect attempts reached for {}", config.hostname);
            break;
        }

        *status.lock().unwrap() = format!("reconnecting ({}/{})", retry, max_retries);
        // ponytail: sleep in small chunks so stop_flag is checked promptly
        let mut elapsed = Duration::ZERO;
        while elapsed < delay {
            if *stop_flag.lock().unwrap() {
                return;
            }
            thread::sleep(Duration::from_millis(200));
            elapsed += Duration::from_millis(200);
        }
        delay = (delay * 2).min(max_delay);
    }

    *status.lock().unwrap() = "disconnected".to_string();
}

fn run_rdp_session(
    config: RdpConfig,
    framebuffer: SharedBuffer,
    stop_flag: Arc<Mutex<bool>>,
    input_rx: Arc<Mutex<std::sync::mpsc::Receiver<InputEvent>>>,
) -> anyhow::Result<()> {
    use ironrdp_connector::credssp::KerberosConfig;

    let addr = format!("{}:{}", config.hostname, config.port);
    let stream = TcpStream::connect(&addr)?;
    stream.set_read_timeout(Some(Duration::from_millis(50)))?;

    let connector_config = Config {
        desktop_size: DesktopSize {
            width: config.width,
            height: config.height,
        },
        desktop_scale_factor: 100,
        enable_tls: true,
        enable_credssp: true,
        credentials: Credentials::UsernamePassword {
            username: config.username.clone(),
            password: config.password.clone(),
        },
        domain: None,
        client_build: 1,
        client_name: "RDPMan".to_string(),
        keyboard_type: ironrdp_pdu::gcc::KeyboardType::IbmEnhanced,
        keyboard_subtype: 0,
        keyboard_functional_keys_count: 12,
        keyboard_layout: 0,
        ime_file_name: String::new(),
        bitmap: None,
        dig_product_id: String::new(),
        client_dir: String::new(),
        alternate_shell: String::new(),
        work_dir: String::new(),
        platform: ironrdp_pdu::rdp::capability_sets::MajorPlatformType::WINDOWS,
        hardware_id: None,
        request_data: None,
        autologon: false,
        enable_audio_playback: true,
        performance_flags: PerformanceFlags::empty(),
        compression_type: None,
        enable_server_pointer: false,
        license_cache: None,
        multitransport_flags: None,
        timezone_info: ironrdp_pdu::rdp::client_info::TimezoneInfo::default(),
        pointer_software_rendering: false,
    };

    let client_addr = "0.0.0.0:0".parse().unwrap();
    let mut connector = ClientConnector::new(connector_config, client_addr)
        .with_static_channel(ironrdp_cliprdr::Cliprdr::new(Box::new(
            crate::clipboard::ClipboardBackend::new(),
        )))
        .with_static_channel(ironrdp_rdpdr::Rdpdr::new(
            Box::new(ironrdp_rdpdr::NoopRdpdrBackend),
            "RDPMan".to_string(),
        ));
    let mut framed = Framed::new(stream);

    // Handshake
    let should_upgrade = connect_begin(&mut framed, &mut connector)?;
    let server_name = ServerName::new(&config.hostname);
    let upgraded = mark_as_upgraded(should_upgrade, &mut connector);
    struct StubNetworkClient;
    impl NetworkClient for StubNetworkClient {
        fn send(&self, _request: &NetworkRequest) -> Result<Vec<u8>, SspiError> {
            Ok(Vec::new())
        }
    }
    let mut network_client = StubNetworkClient;

    let connection_result = connect_finalize(
        upgraded,
        connector,
        &mut framed,
        &mut network_client,
        server_name,
        vec![],
        None,
    )?;

    tracing::info!("RDP connected to {}", config.hostname);

    // Active session
    let mut active_stage = ActiveStageBuilder {
        static_channels: connection_result.static_channels,
        user_channel_id: connection_result.user_channel_id,
        io_channel_id: connection_result.io_channel_id,
        message_channel_id: connection_result.message_channel_id,
        share_id: connection_result.share_id,
        compression_type: connection_result.compression_type,
        enable_server_pointer: false,
        pointer_software_rendering: false,
    }
    .build();

    let mut image = DecodedImage::new(PixelFormat::RgbA32, config.width, config.height);

    let mut write_buf = WriteBuf::new();

    loop {
        if *stop_flag.lock().unwrap() {
            break;
        }

        // Process pending input
        while let Ok(event) = input_rx.lock().unwrap().try_recv() {
            if let Some(events) = encode_input_event(&event) {
                write_buf.clear();
                match active_stage.process_fastpath_input(&mut image, &events) {
                    Ok(outputs) => {
                        for output in outputs {
                            if let ActiveStageOutput::ResponseFrame(data) = output {
                                let _ = framed.write_all(&data);
                            }
                        }
                    }
                    Err(e) => tracing::warn!("Input error: {}", e),
                }
            }
        }

        // Read from network
        match framed.read_pdu() {
            Ok((action, payload)) => match active_stage.process(&mut image, action, &payload) {
                Ok(outputs) => {
                    for output in outputs {
                        match output {
                            ActiveStageOutput::ResponseFrame(data) => {
                                framed.write_all(&data)?;
                            }
                            ActiveStageOutput::GraphicsUpdate(rect) => {
                                let w = config.width as usize;
                                let x0 = rect.left as usize;
                                let y0 = rect.top as usize;
                                let x1 = (rect.right as usize + 1).min(w);
                                let y1 = (rect.bottom as usize + 1).min(config.height as usize);
                                let img = image.data();
                                if let Ok(mut fb) = framebuffer.write() {
                                    for y in y0..y1 {
                                        let src = y * w + x0;
                                        let dst = y * w + x0;
                                        let len = x1 - x0;
                                        if src + len <= img.len() && dst + len <= fb.len() {
                                            for i in 0..len {
                                                fb[dst + i] = u32::from_le_bytes([
                                                    img[src + i * 4],
                                                    img[src + i * 4 + 1],
                                                    img[src + i * 4 + 2],
                                                    img[src + i * 4 + 3],
                                                ]);
                                            }
                                        }
                                    }
                                }
                            }
                            ActiveStageOutput::Terminate(reason) => {
                                tracing::info!("RDP terminated: {:?}", reason);
                                return Ok(());
                            }
                            _ => {}
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Process error: {}", e);
                    break;
                }
            },
            Err(e)
                if e.kind() == io::ErrorKind::TimedOut || e.kind() == io::ErrorKind::WouldBlock =>
            {
                thread::sleep(Duration::from_millis(16));
            }
            Err(e) => {
                tracing::error!("Read error: {}", e);
                break;
            }
        }
    }

    Ok(())
}

fn encode_input_event(event: &InputEvent) -> Option<Vec<FastPathInputEvent>> {
    use ironrdp_pdu::input::fast_path::FastPathInputEvent;
    use ironrdp_pdu::input::fast_path::KeyboardFlags;
    use ironrdp_pdu::input::mouse::{MousePdu, PointerFlags};

    let mut events = Vec::new();

    match event.event_type.as_str() {
        "mouse_move" => {
            events.push(FastPathInputEvent::MouseEvent(MousePdu {
                flags: PointerFlags::MOVE,
                number_of_wheel_rotation_units: 0,
                x_position: event.x,
                y_position: event.y,
            }));
        }
        "mouse_down" => {
            let flags = match event.button {
                1 => PointerFlags::DOWN | PointerFlags::LEFT_BUTTON,
                2 => PointerFlags::DOWN | PointerFlags::RIGHT_BUTTON,
                3 => PointerFlags::DOWN | PointerFlags::MIDDLE_BUTTON_OR_WHEEL,
                _ => return None,
            };
            events.push(FastPathInputEvent::MouseEvent(MousePdu {
                flags,
                number_of_wheel_rotation_units: 0,
                x_position: event.x,
                y_position: event.y,
            }));
        }
        "mouse_up" => {
            let flags = match event.button {
                1 => PointerFlags::LEFT_BUTTON,
                2 => PointerFlags::RIGHT_BUTTON,
                3 => PointerFlags::MIDDLE_BUTTON_OR_WHEEL,
                _ => return None,
            };
            events.push(FastPathInputEvent::MouseEvent(MousePdu {
                flags,
                number_of_wheel_rotation_units: 0,
                x_position: event.x,
                y_position: event.y,
            }));
        }
        "key_down" | "key_up" => {
            let flags = if event.is_down {
                ironrdp_pdu::input::fast_path::KeyboardFlags::empty()
            } else {
                ironrdp_pdu::input::fast_path::KeyboardFlags::RELEASE
            };
            events.push(FastPathInputEvent::KeyboardEvent(
                flags,
                event.key_code as u8,
            ));
        }
        _ => return None,
    }

    Some(events)
}

pub fn new_manager() -> Mutex<SessionManager> {
    Mutex::new(SessionManager::new())
}
