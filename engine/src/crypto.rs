use rcgen::{CertificateParams, KeyPair, DistinguishedName};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::sync::Arc;

pub struct QuicCreds {
    pub certs: Vec<CertificateDer<'static>>,
    pub key: PrivateKeyDer<'static>,
}

/// Generates an on-the-fly self-signed certificate for mTLS
pub fn generate_self_signed() -> Result<QuicCreds, anyhow::Error> {
    let mut params = CertificateParams::default();
    let mut dn = DistinguishedName::new();
    dn.push(rcgen::DnType::CommonName, "p2p-quic-node");
    params.distinguished_name = dn;
    
    // Support TLS 1.3 compatible algos
    let key_pair = KeyPair::generate()?;
    let cert = params.self_signed(&key_pair)?;
    
    Ok(QuicCreds {
        certs: vec![cert.der().to_vec().into()],
        key: PrivateKeyDer::Pkcs8(key_pair.serialize_der().into()),
    })
}

/// A simple Rustls verifier that skips standard CA verification 
/// and blindly accepts the peer's certificate for demo purposes.
/// In production, you would match the peer's certificate hash against a known value.
#[derive(Debug)]
struct SkipVerification;

impl rustls::client::danger::ServerCertVerifier for SkipVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::digitally_signed::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::digitally_signed::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::RSA_PSS_SHA256,
        ]
    }
}

// Mirror verifier for the server side checking the client
#[derive(Debug)]
struct SkipClientVerification;

impl rustls::server::danger::ClientCertVerifier for SkipClientVerification {
    fn verify_client_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::server::danger::ClientCertVerified, rustls::Error> {
        Ok(rustls::server::danger::ClientCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::digitally_signed::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::digitally_signed::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::RSA_PSS_SHA256,
        ]
    }

    fn root_hint_subjects(&self) -> &[rustls::pki_types::DistinguishedName] {
        &[]
    }
}

pub fn configure_quic_client(creds: QuicCreds) -> Result<quinn::ClientConfig, anyhow::Error> {
    let mut crypto = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(SkipVerification))
        .with_client_auth_cert(creds.certs, creds.key)?;
    
    crypto.alpn_protocols = vec![b"p2p-file-transfer".to_vec()];
    let mut client_config = quinn::ClientConfig::new(Arc::new(quinn::crypto::rustls::QuicClientConfig::try_from(crypto)?));
    
    // Optimize transport parameters for fast P2P hole punching if needed
    let mut transport = quinn::TransportConfig::default();
    transport.max_idle_timeout(Some(std::time::Duration::from_secs(10)).try_into().unwrap());
    client_config.transport_config(Arc::new(transport));
    
    Ok(client_config)
}

pub fn configure_quic_server(creds: QuicCreds) -> Result<quinn::ServerConfig, anyhow::Error> {
    let mut crypto = rustls::ServerConfig::builder()
        .with_client_cert_verifier(Arc::new(SkipClientVerification))
        .with_single_cert(creds.certs, creds.key)?;
        
    crypto.alpn_protocols = vec![b"p2p-file-transfer".to_vec()];
    let server_config = quinn::ServerConfig::with_crypto(Arc::new(quinn::crypto::rustls::QuicServerConfig::try_from(crypto)?));
    
    Ok(server_config)
}