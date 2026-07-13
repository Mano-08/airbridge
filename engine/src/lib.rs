uniffi::setup_scaffolding!();
mod constants;
mod stun;
mod types;

use crate::constants::BACKEND_URL;

#[uniffi::export]
fn say_hi() -> String {
    "Hello from Rust!".to_string()
}

#[uniffi::export]
fn create_room() -> String {
    let peer_info = stun::get_peer_address();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(handle_create_room()).unwrap_or_default()
}

async fn handle_create_room() -> Result<String, Box<dyn std::error::Error>> {
    let endpoint = format!("{BACKEND_URL}/api/v1/room/create");
    let resp = reqwest::get(endpoint).await?;
    let room_id = resp.text().await?;
    Ok(room_id)
}

#[uniffi::export]
async fn join_room(room_id: String) -> bool {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(handle_join_room(room_id)).unwrap_or_default()
}

async fn handle_join_room(room_id: String) -> Result<bool, Box<dyn std::error::Error>> {
    let endpoint = format!("{BACKEND_URL}/api/v1/room/join");
    let client = reqwest::Client::new();
    let res = match client.post(endpoint).body(room_id).send().await {
        Ok(r) => r,
        Err(_) => return Ok(false),
    };
    let status_string = match res.text().await {
        Ok(s) => s,
        Err(_) => return Ok(false),
    };
    let status = status_string == "success";
    Ok(status)
}

// main.rs
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

mod crypto;
mod stun_utils;

const LOCAL_PORT: u16 = 50002;
const FILE_SIZE: usize = 10 * 1024; // 10 KB

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage:\n  As Receiver: cargo run -- receiver\n  As Sender:   cargo run -- sender");
        return Ok(());
    }

    // 1. Get our public address via STUN
    println!("Resolving public IP via STUN...");
    match stun_utils::get_public_address(LOCAL_PORT).await {
        Ok(addr) => println!("Your Public Address is: {}", addr),
        Err(e) => println!("STUN Failed (likely symmetric NAT or offline): {}", e),
    }

    let creds = crypto::generate_self_signed()?;

    if args[1] == "receiver" {
        run_receiver(creds).await?;
    } else if args[1] == "sender" {
        // 2. Hardcoded address of friend's device (Replace this with their STUN-reported public address)
        let friend_addr: SocketAddr = "127.0.0.1:50002".parse()?; 
        run_sender(creds, friend_addr).await?;
    }

    Ok(())
}

async fn run_receiver(creds: crypto::QuicCreds) -> Result<(), anyhow::Error> {
    let server_config = crypto::configure_quic_server(creds)?;
    let bind_addr = SocketAddr::from(([0, 0, 0, 0], LOCAL_PORT));
    
    let endpoint = quinn::Endpoint::server(server_config, bind_addr)?;
    println!("Listening for QUIC connections on {}...", bind_addr);

    if let Some(conn) = endpoint.accept().await {
        println!("Incoming connection attempt detected...");
        let connection = conn.await?;
        println!("mTLS Handshake successful! Peer verified.");

        // Accept the incoming bi-directional stream
        let (mut _tx, mut rx) = connection.accept_bi().await?;
        
        let mut buffer = vec![0u8; FILE_SIZE];
        rx.read_exact(&mut buffer).await?;
        
        println!("Successfully received 10KB file via P2P QUIC!");
        println!("First 32 bytes of file data: {:?}", &buffer[..32]);
    }
    Ok(())
}

async fn run_sender(creds: crypto::QuicCreds, friend_addr: SocketAddr) -> Result<(), anyhow::Error> {
    let client_config = crypto::configure_quic_client(creds)?;
    
    // Bind local socket to initiate connection
    let mut endpoint = quinn::Endpoint::client(SocketAddr::from(([0, 0, 0, 0], 0)))?;
    endpoint.set_default_client_config(client_config);

    println!("Connecting to friend at {} via QUIC...", friend_addr);
    // "localhost" acts as the Server Name Indication (SNI) string
    let connection = endpoint.connect(friend_addr, "localhost")?.await?;
    println!("mTLS Handshake successful! Connected to friend.");

    // Open a bi-directional stream to send data
    let (mut tx, mut _rx) = connection.open_bi().await?;

    // Create a mock 10KB file payload
    let mock_file = vec![0x41u8; FILE_SIZE]; // A file full of 'A's
    
    println!("Sending 10KB file...");
    tx.write_all(&mock_file).await?;
    tx.finish().await?; // Close the stream gracefully
    
    println!("File sent successfully!");
    Ok(())
}