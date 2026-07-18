use std::net::IpAddr;

use serde::{Deserialize, Serialize};



#[derive(Debug, uniffi::Error)]
#[uniffi(flat_error)]
pub enum EngineError {
    Generic(String),
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Generic(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for EngineError {}

impl From<anyhow::Error> for EngineError {
    fn from(err: anyhow::Error) -> Self {
        Self::Generic(err.to_string())
    }
}

impl From<std::io::Error> for EngineError {
    fn from(err: std::io::Error) -> Self {
        Self::Generic(err.to_string())
    }
}

impl From<reqwest::Error> for EngineError {
    fn from(err: reqwest::Error) -> Self {
        Self::Generic(err.to_string())
    }
}

pub struct PeerInfo {
    pub ip_address: String,
    pub port: u16,
}

#[derive(Serialize)]
pub struct CreateRoomRequestBody {
    pub passcode: String,
    pub publickey_fingerprint: String,
    pub peer_ip: IpAddr,
    pub peer_port: u16
}

#[derive(Serialize)]
pub struct JoinRoomRequestBody {
    pub passcode: String,
}

#[derive(Serialize, Deserialize)]
pub struct JoinRoomResponseBody {
    publickey_fingerprint: String,
    peer_ip: String,
    peer_port: u16
}