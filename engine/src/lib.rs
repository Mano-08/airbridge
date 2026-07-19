uniffi::setup_scaffolding!();

mod constants;
mod stun;
mod types;
mod crypto;
mod utils;
mod db;

use tokio::runtime::Runtime;
use crate::db::RoomStore;
use crate::types::{EngineError, Room};
use crate::utils::{get_port, handle_create_room, handle_join_room, set_port};

const LOCAL_PORT: u16 = 50_002;

#[uniffi::export]
fn configure_port(port: u16) -> Result<(), EngineError> {
    set_port(port)?;
    Ok(())
}

#[uniffi::export]
fn create_room(passcode: String) -> Result<String, EngineError> {
    let port = get_port()?;
    let peer_info = stun::get_public_address(port)?;
    let rt = Runtime::new()?;
    rt.block_on(handle_create_room(passcode, peer_info, port))
}

#[uniffi::export]
fn join_room(room_id: String, passcode: String) -> Result<bool, EngineError> {
    let _peer_info = stun::get_public_address(LOCAL_PORT)?;
    let rt = Runtime::new()?;
    rt.block_on(handle_join_room(room_id, passcode))
}

#[uniffi::export]
fn get_rooms() -> Result<Vec<Room>, EngineError> {
    let port = get_port()?;
    let store = RoomStore::open(&format!("room_{port}.redb"))?;
    let rooms = store.get_rooms()?;
    Ok(rooms)
}