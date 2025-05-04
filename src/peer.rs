use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct Peer {
    pub addr: SocketAddr,
    pub bitfield: Vec<u8>,
}

impl Peer {
    pub fn new(addr: SocketAddr, bitfield: Vec<u8>) -> Self {
        Self { addr, bitfield }
    }

    pub fn has_index(&self, piece_index: usize) -> bool {
        let byte_index = piece_index / 8;
        let bit_offset = 7 - (piece_index % 8); // BitTorrent uses big-endian bit ordering

        // Make sure the bitfield is large enough to contain this piece
        if byte_index >= self.bitfield.len() {
            return false;
        }

        // Check if the specific bit is set
        (self.bitfield[byte_index] & (1 << bit_offset)) != 0
    }
}
