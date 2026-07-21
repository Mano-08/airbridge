use crate::db::RoomStore;  
use std::convert::identity;
use std::net::SocketAddr;
use std::time::SystemTime;
use reqwest::Client;
use crate::constants::BACKEND_URL;
use crate::types::{CreateRoomRequestBody, EngineError, JoinRoomRequestBody, JoinRoomResponseBody, Room};
use crate::crypto::{generate_self_signed_identity, perform_handshake, perform_server_handshake_entrypoint};
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
                                .post(endpoint)
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
        cert_fingerprint: identity.cert_fingerprint_sha256.clone(),
        peer_ip: _peer_info.ip(),
        peer_port: _peer_info.port()
    };
    let response = client
                                .post(endpoint)
                                .json(&body)
                                .send()
                                .await?;
                            
    let room_id = response.text().await?;
    let room_id_for_room = room_id.clone();
    let room_id_for_db = room_id.clone();
    let room_id_for_handshake = room_id.clone();
    let store = RoomStore::open(&format!("room_{_port}.redb"))?;

    let room = Room {
        room_id: room_id_for_room,
        passcode: body.passcode,
        cert_fingerprint: body.cert_fingerprint,
        peer_ip: body.peer_ip.to_string(),
        peer_port: body.peer_port,
        file_name: String::from(""),
        file_hash: String::from(""),
        total: 0,
        sent: 0,
        created_at: SystemTime::now()
    };

    store.store_room(&room_id_for_db, &room)?;
    tokio::spawn(async move {
        if let Ok(listener) = tokio::net::TcpListener::bind(("0.0.0.0", _port)).await {
            if let Ok((stream, _addr)) = listener.accept().await {
                // hand off `stream` into perform_server_handshake(...)
                let _ = perform_server_handshake_entrypoint(stream, &identity, &room_id_for_handshake, &identity.cert_fingerprint_sha256.clone()).await;
            }
        }
    });
    Ok(room_id)
}