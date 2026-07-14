use serde::Serialize;
use std::net::TcpStream;
use std::net::ToSocketAddrs;
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ServerStatus {
    Online,
    Offline,
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthResult {
    pub id: i64,
    pub hostname: String,
    pub port: i32,
    pub status: ServerStatus,
}

pub fn probe(host: &str, port: u16) -> ServerStatus {
    let addr = match format!("{}:{}", host, port).to_socket_addrs() {
        Ok(mut addrs) => match addrs.next() {
            Some(a) => a,
            None => return ServerStatus::Offline,
        },
        Err(_) => return ServerStatus::Offline,
    };
    match TcpStream::connect_timeout(&addr, Duration::from_secs(3)) {
        Ok(_) => ServerStatus::Online,
        Err(_) => ServerStatus::Offline,
    }
}
