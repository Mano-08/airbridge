use rcgen::{generate_simple_self_signed, CertifiedKey};
use sha2::{Digest, Sha256};
use x509_parser::prelude::*;

use crate::types::EngineError;

/// Holds everything you need: the PEM cert/key to use locally,
/// and the fingerprint to hand to the peer for verification.
pub struct SelfSignedIdentity {
    cert_pem: String,
    key_pem: String,
    cert_der: Vec<u8>,
    /// SHA-256 fingerprint of the DER-encoded certificate (hex string)
    cert_fingerprint_sha256: String,
    /// SHA-256 fingerprint of just the public key (SPKI DER), hex string
    pub pubkey_fingerprint_sha256: String,
}

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