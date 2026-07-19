use crate::db::RoomStore;  
use std::net::SocketAddr;
use reqwest::Client;
use crate::constants::BACKEND_URL;
use crate::types::{CreateRoomRequestBody, EngineError, JoinRoomRequestBody, JoinRoomResponseBody};
use crate::crypto::generate_self_signed_identity;

pub async fn handle_join_room(
    _room_id: String,
    _passcode: String
) -> Result<bool, EngineError> {
    let endpoint = format!("{BACKEND_URL}/api/v1/room/join/{_room_id}");
    let _identity = generate_self_signed_identity(vec!["localhost".to_string()])?;
    let client = Client::new();
    let body = JoinRoomRequestBody {
        passcode: _passcode
    };
    let response = client
                                .post(endpoint)
                                .json(&body)
                                .send()
                                .await?;
                            
    let _response_data: JoinRoomResponseBody = response.json().await?;
    Ok(true)
}

pub async fn handle_create_room(
    _passcode: String,
    _peer_info: SocketAddr
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
    let store = RoomStore::open("rooms.redb")?;
    store.store_room(&room_id, &body)?;
    Ok(room_id)
}