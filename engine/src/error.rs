use x509_parser::error::X509Error;

#[derive(Debug, thiserror::Error, uniffi::Error)]
#[uniffi(flat_error)]
pub enum EngineError {
    // 0
    #[error("request error: {0}")]
    RequestError(#[from] reqwest::Error),

    // 1
    #[error("database error: {0}")]
    DatabaseError(#[from] redb::Error),

    // 2
    #[error("transaction error: {0}")]
    TransactionError(#[from] redb::TransactionError),

    // 3
    #[error("table error: {0}")]
    TableError(#[from] redb::TableError),

    // 4
    #[error("storage error: {0}")]
    StorageError(#[from] redb::StorageError),

    // 5
    #[error("commit error: {0}")]
    CommitError(#[from] redb::CommitError),

    // 6
    #[error("database open error: {0}")]
    DbOpenError(#[from] redb::DatabaseError),

    // 7
    #[error("serialization error: {0}")]
    SerdeError(#[from] serde_json::Error),
    
    // 8
    #[error("extract spki from x509 certificate error: {0}")]
    X509SpkiError(#[from] x509_parser::asn1_rs::Err<X509Error>),

    // 9
    #[error("generate signed certificate error: {0}")]
    GenerateSelfSignedCertificateError(#[from] rcgen::Error),

    // 10
    #[error("i/o error: {0}")]
    IOError(#[from] std::io::Error),

    // 11
    #[error("send message to stun server error: {0}")]
    HandleMessageToStunError(#[from] rtc_shared::error::Error),

    // 12
    #[error("no STUN response received")]
    StunNoResponse,

    // 13
    #[error("port already set error")]
    PortAlreadySet,

    // 14
    #[error("port not set — call set_port first")]
    PortNotSet,

    // 15
    #[error("TLS setup error: {0}")]
    TlsSetupError(String),

    // 16
    #[error("TLS handshake failed: {0}")]
    TlsHandshakeError(String),

    // 17
    #[error("connection error: {0}")]
    ConnectionError(String),

    // 18
    #[error("room not found")]
    RoomNotFound,

    // 19
    #[error("invalid passcode")]
    InvalidPasscode,
}