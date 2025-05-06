#[derive(Debug, Clone)]
pub struct HandshakeMessage {
    pub length: u8,
    pub protocol: [u8; 19],
    pub reserved: [u8; 8],
    pub info_hash: [u8; 20],
    pub peer_id: [u8; 20],
}

impl HandshakeMessage {
    pub fn new(info_hash: [u8; 20], is_magnet: bool) -> Self {
        let peer_id = generate_peer_id();
        let mut reserved = [0u8; 8];
        if is_magnet {
            // 20th bit from last is 1
            reserved[7] = 16;
        }

        HandshakeMessage {
            length: 19,
            protocol: *b"BitTorrent protocol",
            reserved,
            info_hash,
            peer_id: peer_id.as_bytes().try_into().unwrap(),
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

use rand::{rng, Rng};
pub fn generate_peer_id() -> String {
    let mut rng = rng();

    (0..20)
        .map(|_| rng.random_range(0..10).to_string())
        .collect()
}
