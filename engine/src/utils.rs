use crate::db::RoomStore;  
use std::net::SocketAddr;
use std::time::SystemTime;
use reqwest::Client;
use crate::constants::BACKEND_URL;
use crate::types::{CreateRoomRequestBody, EngineError, JoinRoomRequestBody, JoinRoomResponseBody, Room};
use crate::crypto::{generate_self_signed_identity, perform_handshake};
use std::sync::OnceLock;
use crate::db::RoomOperations;

static ENGINE_PORT: OnceLock<u16> = OnceLock::new();

pub fn set_port(port: u16) -> Result<(), EngineError> {
    ENGINE_PORT
        .set(port)
        .map_err(|_| EngineError::PortAlreadySet)
}

pub fn get_port() -> Result<u16, EngineError> {
    ENGINE_PORT
        .get()
        .copied()
        .ok_or(EngineError::PortNotSet)
}

pub async fn handle_join_room(
    _room_id: String,
    _passcode: String
) -> Result<bool, EngineError> {
    let endpoint = format!("{BACKEND_URL}/api/v1/room/join/{_room_id}");
    let client = Client::new();
    let body = JoinRoomRequestBody {
        passcode: _passcode
    };
    let response = client
                                .get(endpoint)
                                .json(&body)
                                .send()
                                .await?;
                            
    let response_data: JoinRoomResponseBody = response.json().await?;
    let joined_room = perform_handshake(response_data).await?;
    Ok(joined_room)
}


pub async fn handle_create_room(
    _passcode: String,
    _peer_info: SocketAddr,
    _port: u16
) -> Result<String, EngineError> {
    let endpoint = format!("{BACKEND_URL}/api/v1/room/create");
    let identity = generate_self_signed_identity(vec!["localhost".to_string()])?;
    let client = Client::new();
    let body = CreateRoomRequestBody {
        passcode: _passcode,
        publickey_fingerprint: identity.pubkey_fingerprint_sha256,
        peer_ip: _peer_info.ip(),
        peer_port: _peer_info.port()
    };
    let response = client
                                .post(endpoint)
                                .json(&body)
                                .send()
                                .await?;
                            
    let room_id = response.text().await?;
    let store = RoomStore::open(&format!("room_{_port}.redb"))?;

    let room = Room {
        room_id: room_id.clone(),
        passcode: body.passcode,
        publickey_fingerprint: body.publickey_fingerprint,
        peer_ip: body.peer_ip.to_string(),
        peer_port: body.peer_port,
        file_name: String::from(""),
        file_hash: String::from(""),
        total: 0,
        sent: 0,
        created_at: SystemTime::now()
    };

    store.store_room(&room_id, &room)?;
    Ok(room_id)
}