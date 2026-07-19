use std::{net::IpAddr, time::SystemTime};
use serde::{Deserialize, Serialize};
use x509_parser::error::X509Error;

#[derive(Debug, thiserror::Error, uniffi::Error)]
#[uniffi(flat_error)]
pub enum EngineError {
    #[error("request error: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("database error: {0}")]
    DatabaseError(#[from] redb::Error),

    #[error("transaction error: {0}")]
    TransactionError(#[from] redb::TransactionError),

    #[error("table error: {0}")]
    TableError(#[from] redb::TableError),

    #[error("storage error: {0}")]
    StorageError(#[from] redb::StorageError),

    #[error("commit error: {0}")]
    CommitError(#[from] redb::CommitError),

    #[error("database open error: {0}")]
    DbOpenError(#[from] redb::DatabaseError),

    #[error("serialization error: {0}")]
    SerdeError(#[from] serde_json::Error),
    
    #[error("extract spki from x509 certificate error: {0}")]
    X509SpkiError(#[from] x509_parser::asn1_rs::Err<X509Error>),

    #[error("generate signed certificate error: {0}")]
    GenerateSelfSignedCertificateError(#[from] rcgen::Error),

    #[error("i/o error: {0}")]
    IOError(#[from] std::io::Error),

    #[error("send message to stun server error: {0}")]
    HandleMessageToStunError(#[from] rtc_shared::error::Error),

    #[error("no STUN response received")]
    StunNoResponse,

    #[error("port already set error")]
    PortAlreadySet,

    #[error("port not set — call set_port first")]
    PortNotSet,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateRoomRequestBody {
    pub passcode: String,
    pub publickey_fingerprint: String,
    pub peer_ip: IpAddr,
    pub peer_port: u16
} 

#[derive(Debug, Serialize, Deserialize, Clone, uniffi::Record)]
pub struct Room {
    pub room_id: String,
    pub passcode: String,
    pub publickey_fingerprint: String,
    pub peer_ip: String,
    pub peer_port: u16,
    pub file_name: String,
    pub file_hash: String,
    pub sent: u32,
    pub total: u32,
    pub created_at: SystemTime,
} 
 

#[derive(Serialize)]
pub struct JoinRoomRequestBody {
    pub passcode: String,
}


#[derive(Debug, Deserialize)]
pub struct JoinRoomResponseBody {
    publickey_fingerprint: String,
    peer_ip: String,
    peer_port: u16
}