uniffi::setup_scaffolding!();

mod constants;
mod stun;
mod types;
mod crypto;
mod utils;
mod db;

use tokio::runtime::Runtime;
use crate::types::{EngineError};
use crate::utils::{handle_create_room, handle_join_room};

const LOCAL_PORT: u16 = 50_002;

#[uniffi::export]
fn create_room(passcode: String) -> Result<String, EngineError> {
    let peer_info = stun::get_public_address(LOCAL_PORT)?;
    let rt = Runtime::new()?;
    rt.block_on(handle_create_room(passcode, peer_info))
}

#[uniffi::export]
fn join_room(room_id: String, passcode: String) -> Result<bool, EngineError> {
    let _peer_info = stun::get_public_address(LOCAL_PORT)?;
    let rt = Runtime::new()?;
    rt.block_on(handle_join_room(room_id, passcode))
}
