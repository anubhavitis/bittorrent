use std::{
    io::{Read, Write},
    net::{Shutdown, SocketAddr, TcpStream},
    path::PathBuf,
};

use crate::torrent::Torrent;

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
    let handshake_message = HandshakeMessage::new(torrent.get_info_hash());

    let mut tcp_stream = match TcpStream::connect(peer) {
        Ok(stream) => stream,
        Err(e) => panic!("Failed to connect to peer: {}", e),
    };

    eprintln!("Writing handshake message to peer");
    let handshake_bytes = handshake_message.to_bytes();
    tcp_stream
        .write_all(&handshake_bytes)
        .expect("write handshake");

    eprintln!("Reading handshake message from peer");
    let mut buffer = [0u8; 68];
    tcp_stream.read_exact(&mut buffer).unwrap();

    eprintln!("Shutting down TCP stream");
    tcp_stream.shutdown(Shutdown::Both).unwrap();

    eprintln!("Peer Id: {}", hex::encode(&handshake_bytes[48..68]));
    println!("Peer Id: {}", hex::encode(&buffer[48..68]));
}
