use std::net::IpAddr;
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
}

// impl std::fmt::Display for EngineError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Self::RequestError(error) => write!(f, "{error}"),
//             Self::DatabaseError(error) => write!(f, "{error}")
//         }
//     }
// }

// impl std::error::Error for EngineError {}

// impl From<anyhow::Error> for EngineError {
//     fn from(err: anyhow::Error) -> Self {
//         Self::Generic(err.to_string())
//     }
// }

// impl From<std::io::Error> for EngineError {
//     fn from(err: std::io::Error) -> Self {
//         Self::Generic(err.to_string())
//     }
// }

// impl From<redb::Error> for EngineError {
//     fn from(err: redb::Error) -> Self {
//         EngineError::DatabaseError(err)
//     }
// }

// impl From<reqwest::Error> for EngineError {
//     fn from(err: reqwest::Error) -> Self {
//         EngineError::RequestError(err)
//     }
// }


pub struct PeerInfo {
    pub ip_address: String,
    pub port: u16,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
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


#[derive(Debug, Deserialize)]
pub struct JoinRoomResponseBody {
    publickey_fingerprint: String,
    peer_ip: String,
    peer_port: u16
}