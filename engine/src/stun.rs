// stun_utils.rs
use std::net::{SocketAddr, UdpSocket};
use stun::message::{Message, BINDING_REQUEST, Getter};
use stun::xoraddr::XorMappedAddress;

pub async fn get_public_address(local_port: u16) -> Result<SocketAddr, anyhow::Error> {
    // Bind to all interfaces on the specified port
    let socket = UdpSocket::bind(format!("0.0.0.0:{}", local_port))?;
    socket.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;

    // Connect to a public Google STUN server
    let stun_server: SocketAddr = "74.125.250.129:19302".parse()?; // stun.l.google.com:19302
    socket.connect(stun_server)?;

    // Construct STUN binding request
    let mut msg = Message::new();
    msg.build(&[Box::new(BINDING_REQUEST)])?;

    // Send request
    socket.send(msg.raw())?;

    // Receive response
    let mut buf = [0u8; 1024];
    let len = socket.recv(&mut buf)?;

    let mut response = Message::new();
    response.set_raw(buf[..len].to_vec());
    response.decode()?;

    // Extract XOR Mapped Address (our public IP and Port)
    let mut xor_addr = XorMappedAddress::default();
    xor_addr.get_from(&response)?;

    let public_addr = SocketAddr::new(xor_addr.ip, xor_addr.port);
    Ok(public_addr)
}