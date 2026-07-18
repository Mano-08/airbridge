use std::net::{SocketAddr, UdpSocket};
use std::time::Instant;

use bytes::BytesMut;
use rtc_shared::{TaggedBytesMut, TransportContext, TransportProtocol};
use rtc_stun::client::ClientBuilder;
use rtc_stun::message::{Getter, Message, BINDING_REQUEST, TransactionId};
use rtc_stun::xoraddr::XorMappedAddress;
use sansio::Protocol;

pub fn get_public_address(local_port: u16) -> Result<SocketAddr, anyhow::Error> {
    let server = String::from("stun.l.google.com:19302");
    let conn = UdpSocket::bind(format!("0.0.0.0:{local_port}"))?;
    conn.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;
    conn.connect(server)?;

    let mut client = ClientBuilder::new().build(
        conn.local_addr()?,
        conn.peer_addr()?,
        TransportProtocol::UDP,
    )?;

    let mut msg = Message::new();
    msg.build(&[Box::<TransactionId>::default(), Box::new(BINDING_REQUEST)])?;
    client.handle_write(msg)?;
    while let Some(transmit) = client.poll_write() {
        conn.send(&transmit.message)?;
    }

    let mut buf = vec![0u8; 1500];
    let n = conn.recv(&mut buf)?;
    client.handle_read(TaggedBytesMut {
        now: Instant::now(),
        transport: TransportContext {
            local_addr: conn.local_addr()?,
            peer_addr: conn.peer_addr()?,
            transport_protocol: TransportProtocol::UDP,
            ecn: None,
        },
        message: BytesMut::from(&buf[..n]),
    })?;

    let public_addr = if let Some(event) = client.poll_event() {
        let msg = event.result?;
        let mut xor_addr = XorMappedAddress::default();
        xor_addr.get_from(&msg)?;
        SocketAddr::new(xor_addr.ip, xor_addr.port)
    } else {
        anyhow::bail!("no STUN response received");
    };

    client.close()?;

    Ok(public_addr)
}
