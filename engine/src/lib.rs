uniffi::setup_scaffolding!();

mod constants;
mod stun;
mod types;
mod crypto;
mod utils;
mod db;

use tokio::runtime::Runtime;
use crate::db::RoomStore;
use crate::db::RoomOperations;
use crate::types::{EngineError, Room};
use crate::utils::{get_port, handle_create_room, handle_join_room, set_port};
use std::sync::OnceLock;

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

fn get_runtime() -> Result<&'static Runtime, EngineError> {
    if let Some(rt) = RUNTIME.get() {
        return Ok(rt);
    }
    let rt = Runtime::new().map_err(|e| EngineError::ConnectionError(e.to_string()))?;
    let _ = RUNTIME.set(rt);
    Ok(RUNTIME.get().unwrap())
}

const LOCAL_PORT: u16 = 50_002;
use simplelog::*;
use std::fs::File;
use std::net::SocketAddr;

#[uniffi::export]
fn debug_clear_room_db() -> Result<String, EngineError> {
    let port = get_port()?;
    let db_path = format!("room_{port}.redb");

    let resolved_path = std::env::current_dir()
        .map(|d| d.join(&db_path))
        .unwrap_or_else(|_| std::path::PathBuf::from(&db_path));

    log::info!("[debug] Attempting to delete db at: {:?}", resolved_path);

    match std::fs::remove_file(&resolved_path) {
        Ok(_) => {
            let msg = format!("Deleted: {:?}", resolved_path);
            log::info!("[debug] {}", msg);
            Ok(msg)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let msg = format!("No file found at: {:?}", resolved_path);
            log::warn!("[debug] {}", msg);
            Ok(msg)
        }
        Err(e) => {
            let msg = format!("Failed to delete {:?}: {e}", resolved_path);
            log::error!("[debug] {}", msg);
            Err(EngineError::NotFound(msg))
        }
    }
}

#[uniffi::export]
fn configure_logging(log_path: String) -> Result<(), EngineError> {
    let log_file = File::create(&log_path)
        .map_err(|e| EngineError::TlsSetupError(format!("failed to create log file: {e}")))?;

    WriteLogger::init(LevelFilter::Debug, Config::default(), log_file)
        .map_err(|e| EngineError::TlsSetupError(format!("failed to init logger: {e}")))?;

    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| EngineError::TlsSetupError("failed to install rustls crypto provider".into()))?;

    log::info!("Logging initialized at {log_path}");
    Ok(())
}

#[uniffi::export]
fn configure_port(port: u16) -> Result<(), EngineError> {
    log::info!("[configure_port] Setting port to {}", port);
    set_port(port)?;
    Ok(())
}

#[uniffi::export]
fn create_room(passcode: String) -> Result<String, EngineError> {
    log::info!("[create_room] Creating room with passcode");
    let port = get_port()?;
    let peer_info = SocketAddr::new("127.0.0.1".parse().unwrap(), port);
    let rt = get_runtime()?;  // shared, persistent runtime — never dropped
    let result = rt.block_on(handle_create_room(passcode, peer_info, port));
    match &result {
        Ok(room_id) => log::info!("[create_room] Room created successfully: {}", room_id),
        Err(e) => log::error!("[create_room] Room creation failed: {:?}", e),
    }
    result
}

#[uniffi::export]
fn join_room(room_id: String, passcode: String) -> Result<bool, EngineError> {
    log::info!("[join] starting, room_id={room_id}");
    let peer_info = match stun::get_public_address(LOCAL_PORT) {
        Ok(info) => {
            log::debug!("[join] got peer_data: {:?}", info);
            info
        }
        Err(e) => {
            log::error!("[join] STUN public address retrieval failed: {:?}", e);
            return Err(e);
        }
    };
    let rt = get_runtime()?;
    // If handle_join_room contains a TCP connection, error logging is expected there.
    let result = rt.block_on(handle_join_room(room_id.clone(), passcode));
    match &result {
        Ok(success) => log::info!("[join] join_room completed: {}", success),
        Err(e) => log::error!("[join] TCP connect failed: {:?}", e),
    }
    result
}

#[uniffi::export]
fn get_rooms() -> Result<Vec<Room>, EngineError> {
    let port = get_port()?;
    log::info!("[get_rooms] Getting rooms for port {}", port);
    let store = RoomStore::open(&format!("room_{port}.redb"))?;
    let rooms = store.get_rooms()?;
    log::debug!("[get_rooms] Fetched {} rooms", rooms.len());
    Ok(rooms)
}