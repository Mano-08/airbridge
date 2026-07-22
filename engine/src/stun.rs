use std::net::{SocketAddr, UdpSocket};
use std::time::Instant;

use bytes::BytesMut;
use rtc_shared::{TaggedBytesMut, TransportContext, TransportProtocol};
use rtc_stun::client::ClientBuilder;
use rtc_stun::message::{Getter, Message, BINDING_REQUEST, TransactionId};
use rtc_stun::xoraddr::XorMappedAddress;
use sansio::Protocol;

use crate::types::EngineError;

pub fn get_public_address(local_port: u16) -> Result<SocketAddr, EngineError> {
    log::info!("[stun] starting, local_port={local_port}");
    let server = String::from("stun.l.google.com:19302");

    let conn = match UdpSocket::bind(format!("0.0.0.0:{local_port}")) {
        Ok(socket) => socket,
        Err(e) => {
            log::error!("[stun] UDP bind failed: {}", e);
            return Err(EngineError::IOError(e));
        }
    };

    if let Err(e) = conn.set_read_timeout(Some(std::time::Duration::from_secs(5))) {
        log::error!("[stun] set_read_timeout failed: {}", e);
        return Err(EngineError::IOError(e));
    }
    if let Err(e) = conn.connect(&server) {
        log::error!("[stun] connect to server {} failed: {}", server, e);
        return Err(EngineError::IOError(e));
    }

    let mut client = match ClientBuilder::new().build(
        conn.local_addr().map_err(|e| {
            log::error!("[stun] get local_addr failed: {}", e);
            EngineError::IOError(e)
        })?,
        conn.peer_addr().map_err(|e| {
            log::error!("[stun] get peer_addr failed: {}", e);
            EngineError::IOError(e)
        })?,
        TransportProtocol::UDP,
    ) {
        Ok(c) => c,
        Err(e) => {
            log::error!("[stun] client build failed: {}", e);
            return Err(EngineError::HandleMessageToStunError(e));
        }
    };

    let mut msg = Message::new();
    if let Err(e) = msg.build(&[Box::<TransactionId>::default(), Box::new(BINDING_REQUEST)]) {
        log::error!("[stun] message build failed: {}", e);
        return Err(EngineError::HandleMessageToStunError(e));
    }
    if let Err(e) = client.handle_write(msg) {
        log::error!("[stun] client handle_write failed: {}", e);
        return Err(EngineError::HandleMessageToStunError(e));
    }
    while let Some(transmit) = client.poll_write() {
        if let Err(e) = conn.send(&transmit.message) {
            log::error!("[stun] conn.send failed: {}", e);
            return Err(EngineError::IOError(e));
        }
    }

    let mut buf = vec![0u8; 1500];
    let n = match conn.recv(&mut buf) {
        Ok(n) => {
            log::debug!("[stun] received {} bytes from STUN server", n);
            n
        },
        Err(e) => {
            log::error!("[stun] receiving response from STUN server failed: {}", e);
            return Err(EngineError::IOError(e));
        }
    };

    let tagged = TaggedBytesMut {
        now: Instant::now(),
        transport: TransportContext {
            local_addr: conn.local_addr().map_err(|e| {
                log::error!("[stun] get local_addr failed: {}", e);
                EngineError::IOError(e)
            })?,
            peer_addr: conn.peer_addr().map_err(|e| {
                log::error!("[stun] get peer_addr failed: {}", e);
                EngineError::IOError(e)
            })?,
            transport_protocol: TransportProtocol::UDP,
            ecn: None,
        },
        message: BytesMut::from(&buf[..n]),
    };

    if let Err(e) = client.handle_read(tagged) {
        log::error!("[stun] client handle_read failed: {}", e);
        return Err(EngineError::HandleMessageToStunError(e));
    }

    let public_addr = if let Some(event) = client.poll_event() {
        let msg = match event.result {
            Ok(msg) => {
                log::debug!("[stun] received event: {:?}", msg);
                msg
            }
            Err(e) => {
                log::error!("[stun] event result error: {}", e);
                return Err(EngineError::HandleMessageToStunError(e));
            }
        };
        let mut xor_addr = XorMappedAddress::default();
        if let Err(e) = xor_addr.get_from(&msg) {
            log::error!("[stun] get_from msg failed: {}", e);
            return Err(EngineError::HandleMessageToStunError(e));
        }
        let addr = SocketAddr::new(xor_addr.ip, xor_addr.port);
        log::info!("[stun] public address determined: {}", addr);
        addr
    } else {
        log::error!("[stun] no STUN response received");
        return Err(EngineError::StunNoResponse);
    };

    if let Err(e) = client.close() {
        log::error!("[stun] client close failed: {}", e);
    }

    Ok(public_addr)
}
