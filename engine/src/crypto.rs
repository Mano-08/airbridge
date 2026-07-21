use rcgen::{generate_simple_self_signed, CertifiedKey};
use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
use sha2::{Digest, Sha256};
use tokio_rustls::server::TlsStream;
use crate::db::{RoomOperations, RoomStore};
use crate::utils::get_port;
use x509_parser::prelude::*;
use crate::types::{EngineError, JoinRoomResponseBody, SelfSignedIdentity};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName, UnixTime};
use rustls::client::danger::{ServerCertVerifier, ServerCertVerified, HandshakeSignatureValid};
use rustls::{DigitallySignedStruct, SignatureScheme};
use tokio::net::TcpStream;
use tokio_rustls::{TlsAcceptor, TlsConnector};

pub fn generate_self_signed_identity(subject_alt_names: Vec<String>) -> Result<SelfSignedIdentity, EngineError> {
    // Generates a self-signed cert + keypair for the given SANs (e.g. "localhost", "peerA.local")
    let CertifiedKey { cert, signing_key } = generate_simple_self_signed(subject_alt_names)?;

    let cert_pem = cert.pem();
    let key_pem = signing_key.serialize_pem();
    let cert_der = cert.der().to_vec();

    // Fingerprint of the whole certificate (common for TLS cert pinning)
    let cert_fingerprint_sha256 = sha256_hex(&cert_der);

    // Fingerprint of just the SubjectPublicKeyInfo (public key only, no cert wrapper)
    // Useful when you want to pin the key even if the cert is regenerated/reissued.
    let spki_der = extract_spki_der(&cert_der)?;
    let pubkey_fingerprint_sha256 = sha256_hex(&spki_der);

    Ok(SelfSignedIdentity {
        cert_pem,
        key_pem,
        cert_der,
        cert_fingerprint_sha256,
        pubkey_fingerprint_sha256,
    })
}

fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Parses the DER certificate to pull out the SubjectPublicKeyInfo bytes.
/// Uses `x509-parser` under the hood for correctness instead of hand-rolling ASN.1 parsing.
fn extract_spki_der(cert_der: &[u8]) -> Result<Vec<u8>, EngineError> {
    let (_, cert) = X509Certificate::from_der(cert_der)?;
    Ok(cert.public_key().raw.to_vec())
}


/// Verifier that ignores normal CA chain validation and instead checks
/// the presented certificate's SHA-256 fingerprint against a pinned value.
#[derive(Debug)]
struct FingerprintPinningVerifier {
    expected_fingerprint_hex: String,
}

impl ClientCertVerifier for FingerprintPinningVerifier {
    fn offer_client_auth(&self) -> bool {
        true
    }

    fn client_auth_mandatory(&self) -> bool {
        true
    }

    fn root_hint_subjects(&self) -> &[rustls::DistinguishedName] {
        &[]
    }

    fn verify_client_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _now: UnixTime,
    ) -> Result<ClientCertVerified, rustls::Error> {
        self.check(end_entity.as_ref())?;
        Ok(ClientCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![SignatureScheme::ECDSA_NISTP256_SHA256]
    }
}

impl FingerprintPinningVerifier {
    fn check(&self, cert_der: &[u8]) -> Result<(), rustls::Error> {
        let mut hasher = Sha256::new();
        hasher.update(cert_der);
        let actual = hex::encode(hasher.finalize());

        if actual.eq_ignore_ascii_case(&self.expected_fingerprint_hex) {
            Ok(())
        } else {
            Err(rustls::Error::General(format!(
                "certificate fingerprint mismatch: expected {}, got {}",
                self.expected_fingerprint_hex, actual
            )))
        }
    }
}

// Used on the TLS client side (when we're the one dialing out / joining)
impl ServerCertVerifier for FingerprintPinningVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        self.check(end_entity.as_ref())?;
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        // Since our self-signed certs use ECDSA P-256 (rcgen's default)
        vec![SignatureScheme::ECDSA_NISTP256_SHA256]
    }
}

fn load_private_key(key_pem: &str) -> Result<PrivateKeyDer<'static>, EngineError> {
    let mut reader = std::io::Cursor::new(key_pem.as_bytes());
    let key = rustls_pemfile::private_key(&mut reader)
        .map_err(|_| EngineError::TlsSetupError("failed to parse private key".into()))?
        .ok_or_else(|| EngineError::TlsSetupError("no private key found in PEM".into()))?;
    Ok(key)
}

async fn perform_client_handshake(
    stream: TcpStream,
    identity: &SelfSignedIdentity,
    expected_peer_fingerprint: &str,
) -> Result<bool, EngineError> {
    let cert_chain = vec![CertificateDer::from(identity.cert_der.to_vec())];
    let key = load_private_key(&identity.key_pem)?;

    let verifier = Arc::new(FingerprintPinningVerifier {
        expected_fingerprint_hex: expected_peer_fingerprint.to_string(),
    });

    let config = rustls::ClientConfig::builder()
        .dangerous() // required whenever bypassing normal CA-chain verification
        .with_custom_certificate_verifier(verifier)
        .with_client_auth_cert(cert_chain, key)
        .map_err(|e| EngineError::TlsSetupError(e.to_string()))?;

    let connector = TlsConnector::from(Arc::new(config));

    // ServerName is required by the API but functionally unused here, since
    // we're validating identity via fingerprint, not hostname matching.
    let server_name = ServerName::try_from("localhost")
        .map_err(|_| EngineError::TlsSetupError("invalid server name".into()))?
        .to_owned();

    let tls_stream = connector
        .connect(server_name, stream)
        .await
        .map_err(|e| EngineError::TlsHandshakeError(e.to_string()))?;

    // Handshake succeeded AND the peer's cert fingerprint matched — mutual trust established.
    let _ = tls_stream; // keep/use this stream for the actual file transfer that follows
    Ok(true)
}

pub async fn perform_handshake(
    peer_data: JoinRoomResponseBody
) -> Result<bool, EngineError> {
    let identity = generate_self_signed_identity(vec!["localhost".to_string()])?;

    let addr = format!("{}:{}", peer_data.peer_ip, peer_data.peer_port);
    let expected_fingerprint = peer_data.cert_fingerprint.clone();

    let stream = TcpStream::connect(&addr)
        .await
        .map_err(|e| EngineError::ConnectionError(e.to_string()))?;
    perform_client_handshake(stream, &identity, &expected_fingerprint).await
}

async fn perform_server_handshake_raw(
    stream: TcpStream,
    identity: &SelfSignedIdentity,
    expected_peer_fingerprint: &str,
) -> Result<tokio_rustls::server::TlsStream<TcpStream>, EngineError> {
    let cert_chain = vec![CertificateDer::from(identity.cert_der.to_vec())];
    let key = load_private_key(&identity.key_pem)?;

    let verifier = Arc::new(FingerprintPinningVerifier {
        expected_fingerprint_hex: expected_peer_fingerprint.to_string(),
    });

    let config = rustls::ServerConfig::builder()
        .with_client_cert_verifier(verifier)
        .with_single_cert(cert_chain, key)
        .map_err(|e| EngineError::TlsSetupError(e.to_string()))?;

    let acceptor = TlsAcceptor::from(Arc::new(config));

    acceptor
        .accept(stream)
        .await
        .map_err(|e| EngineError::TlsHandshakeError(e.to_string()))
}

pub async fn perform_server_handshake_entrypoint(
    stream: TcpStream,
    identity: &SelfSignedIdentity,
    room_id: &str,
    expected_client_fingerprint: &str,
) -> Result<bool, EngineError> {
    // Steps 7-9: mTLS handshake — rustls handles ClientHello/ServerHello
    // internally; our FingerprintPinningVerifier checks Peer B's cert
    // fingerprint as part of this call (step 9).
    let mut tls_stream: TlsStream<TcpStream> = perform_server_handshake_raw(stream, identity, expected_client_fingerprint).await?;

    // Step 10: read the passcode Peer B sends over the encrypted stream.
    // Using a simple length-prefixed message: 4-byte big-endian length,
    // followed by that many bytes of UTF-8 passcode.
    let mut len_buf = [0u8; 4];
    tls_stream
        .read_exact(&mut len_buf)
        .await
        .map_err(|e| EngineError::TlsHandshakeError(format!("failed to read passcode length: {e}")))?;
    let msg_len = u32::from_be_bytes(len_buf) as usize;

    // Sanity check to avoid a malicious/buggy peer requesting a huge allocation
    if msg_len > 1024 {
        return Err(EngineError::TlsHandshakeError("passcode message too large".into()));
    }

    let mut passcode_buf = vec![0u8; msg_len];
    tls_stream
        .read_exact(&mut passcode_buf)
        .await
        .map_err(|e| EngineError::TlsHandshakeError(format!("failed to read passcode: {e}")))?;
    let received_passcode = String::from_utf8(passcode_buf)
        .map_err(|_| EngineError::TlsHandshakeError("passcode was not valid UTF-8".into()))?;

    let port = get_port()?;
    // Step 11: validate against the passcode stored for this room.
    let store = RoomStore::open(&format!("room_{port}.redb"))?;
    let room = store
        .get_room(room_id)?
        .ok_or(EngineError::RoomNotFound)?;

    let is_valid = room.passcode == received_passcode;

    // Let Peer B know whether they're in — a single byte, 1 = ok, 0 = rejected.
    let response_byte: [u8; 1] = if is_valid { [1] } else { [0] };
    tls_stream
        .write_all(&response_byte)
        .await
        .map_err(|e| EngineError::TlsHandshakeError(format!("failed to send auth result: {e}")))?;
    tls_stream
        .flush()
        .await
        .map_err(|e| EngineError::TlsHandshakeError(format!("failed to flush stream: {e}")))?;

    if !is_valid {
        return Err(EngineError::InvalidPasscode);
    }

    // Step 12: peer A and peer B can now transfer files over `tls_stream`.
    // TODO: hand `tls_stream` off to your file-transfer logic here instead
    // of letting it drop — currently the stream closes once this function returns.

    Ok(true)
}