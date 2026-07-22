use crate::db::RoomStore;
use std::convert::identity;
use std::net::SocketAddr;
use std::time::SystemTime;
use reqwest::Client;
use crate::constants::BACKEND_URL;
use crate::types::{
    CreateRoomRequestBody, EngineError, JoinRoomRequestBody, JoinRoomResponseBody, Room,
};
use crate::crypto::{generate_self_signed_identity, perform_join_passcode_send, perform_server_handshake_entrypoint, perform_server_passcode_check};
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
    room_id: String,
    passcode: String,
) -> Result<bool, EngineError> {
    // Log the start of the join handler.
    log::info!("[join] starting, room_id={room_id}");
    let endpoint = format!("{BACKEND_URL}/api/v1/room/join/{room_id}");
    let client = Client::new();
    let passcode_clone_for_req = passcode.clone();
    let body = JoinRoomRequestBody {
        passcode: passcode_clone_for_req,
    };
    let response = client
        .post(endpoint)
        .json(&body)
        .send()
        .await?;

    let peer_data: JoinRoomResponseBody = response.json().await?;
    log::debug!("[join] got peer_data: {:?}", peer_data);


    let joined_room = match perform_join_passcode_send(peer_data, &passcode).await {
        Ok(val) => val,
        Err(e) => {
            // log error if connection failed at TCP/connect stage
            log::error!("[join] TCP connect failed: {e}");
            return Err(e);
        }
    };
    Ok(joined_room)
}

pub async fn handle_create_room(
    _passcode: String,
    _peer_info: SocketAddr,
    _port: u16,
) -> Result<String, EngineError> {
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", _port))
        .await
        .map_err(|e| EngineError::ConnectionError(format!("failed to bind port {_port}: {e}")))?;

    let endpoint = format!("{BACKEND_URL}/api/v1/room/create");
    // let identity = generate_self_signed_identity(vec!["localhost".to_string()])?;
    let client = Client::new();
    let body = CreateRoomRequestBody {
        passcode: _passcode,
        peer_ip: _peer_info.ip(),
        peer_port: _peer_info.port(),
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
        peer_ip: body.peer_ip.to_string(),
        peer_port: body.peer_port,
        file_name: String::from(""),
        file_hash: String::from(""),
        total: 0,
        sent: 0,
        created_at: SystemTime::now(),
        connected: false,
    };

    store.store_room(&room_id_for_db, &room)?;
    let port_for_task = _port;

    tokio::spawn(async move {
        if let Ok((stream, addr)) = listener.accept().await {
            log::info!("[host] accepted connection from {addr}");
            match perform_server_passcode_check(stream, &room_id_for_handshake).await {
                Ok(_) => {
                    log::info!("[host] passcode check succeeded for room {}", room_id_for_handshake);
                    match RoomStore::open(&format!("room_{port_for_task}.redb")) {
                        Ok(store) => {
                            if let Err(e) = store.mark_room_connected(&room_id_for_handshake) {
                                log::error!("[host] failed to mark room connected: {e:?}");
                            }
                        }
                        Err(e) => log::error!("[host] failed to reopen store: {e:?}"),
                    }
                }
                Err(e) => {
                    log::error!("[host] passcode check failed for room {}: {e:?}", room_id_for_handshake);
                }
            }
        }
    });

    Ok(room_id)
}