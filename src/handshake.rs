use std::{
    io::{Read, Write},
    net::{Shutdown, SocketAddr, TcpStream},
    path::PathBuf,
};

use crate::torrent::Torrent;

#[derive(Debug)]
pub struct HandshakeMessage {
    pub length: u8,
    pub protocol: [u8; 19],
    pub reserved: [u8; 8],
    pub info_hash: [u8; 20],
    pub peer_id: [u8; 20],
}

impl HandshakeMessage {
    pub fn new(info_hash: [u8; 20]) -> Self {
        HandshakeMessage {
            length: 19,
            protocol: *b"BitTorrent protocol",
            reserved: [0; 8],
            info_hash,
            peer_id: *b"00112233445566778899",
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        HandshakeMessage {
            length: bytes[0],
            protocol: bytes[1..20].try_into().unwrap(),
            reserved: bytes[20..28].try_into().unwrap(),
            info_hash: bytes[28..48].try_into().unwrap(),
            peer_id: bytes[48..68].try_into().unwrap(),
        }
    }

    pub fn to_bytes(&self) -> [u8; 68] {
        let mut bytes = [0u8; 68];
        bytes[0] = self.length;
        bytes[1..20].copy_from_slice(b"BitTorrent protocol");
        bytes[20..28].copy_from_slice(&self.reserved);
        bytes[28..48].copy_from_slice(&self.info_hash);
        bytes[48..68].copy_from_slice(&self.peer_id);
        bytes
    }
}

pub async fn handshake(file_name: &PathBuf, peer: SocketAddr) {
    let torrent = Torrent::new(file_name);
    let info_hash = torrent.get_info_hash();
    let handshake_message = HandshakeMessage::new(info_hash);

    eprintln!("Handshake message: {:?}", handshake_message);

    let mut tcp_stream = match TcpStream::connect(peer) {
        Ok(stream) => stream,
        Err(e) => panic!("Failed to connect to peer: {}", e),
    };

    eprintln!("Writing handshake message to peer");
    let handshake_bytes = handshake_message.to_bytes();
    tcp_stream
        .write_all(&handshake_bytes)
        .expect("write handshake");

    let mut buffer = [0u8; 68];
    tcp_stream.read_exact(&mut buffer).unwrap();
    // let response = HandshakeMessage::from_bytes(&buffer);

    eprintln!("Peer Id: {}", hex::encode(handshake_message.peer_id));

    eprintln!("Shutting down TCP stream");
    tcp_stream.shutdown(Shutdown::Both).unwrap();
}
