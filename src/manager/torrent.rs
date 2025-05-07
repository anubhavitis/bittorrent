use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use sha1::{Digest, Sha1};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Torrent {
    pub announce: String,
    pub info: Info,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Info {
    pub name: String,
    pub length: u32,
    pub pieces: ByteBuf,
    #[serde(rename = "piece length")]
    pub piece_length: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct TrackerResponse {
    interval: u32,
    peers: ByteBuf,
}

impl Torrent {
    pub fn new(file_name: &PathBuf) -> Self {
        let file = std::fs::read(file_name).expect("Failed to read the file");
        let torrent: Torrent = serde_bencode::from_bytes(&file).unwrap();
        torrent
    }

    pub fn get_info_hash(&self) -> [u8; 20] {
        let mut hasher = Sha1::new();
        let info_bytes = serde_bencode::to_bytes(&self.info).unwrap();
        hasher.update(&info_bytes);
        let hash = hasher.finalize();
        hash.try_into().expect("Failed to convert hash to array")
    }

    pub async fn get_peers(&self) -> Result<Vec<SocketAddr>, Box<dyn std::error::Error>> {
        let info_hash = self.get_info_hash();
        let url_encoded_info_hash = urlencoding::encode_binary(&info_hash).to_string();

        let url_params = serde_json::json!({
            "peer_id": "01012323454567678989",
            "port": 6881,
            "uploaded": 1,
            "downloaded": 1,
            "left": self.info.length,
            "compact": 1
        });

        let encoded_url_params = serde_urlencoded::to_string(&url_params)?;

        let url = format!(
            "{}?{}&info_hash={}",
            self.announce, encoded_url_params, url_encoded_info_hash
        );
        dbg!(&url);

        // Send request to tracker and parse response
        let tracker_response = reqwest::get(url.as_str()).await?;
        let tracker_response_bytes = tracker_response.bytes().await?;
        let tracker_response: TrackerResponse = serde_bencode::from_bytes(&tracker_response_bytes)?;

        let mut peers = Vec::new();
        let mut i = 0;
        while i < tracker_response.peers.len() {
            if i + 6 <= tracker_response.peers.len() {
                let peer = &tracker_response.peers[i..i + 6];
                let formatted_peer = SocketAddr::new(
                    IpAddr::V4(Ipv4Addr::new(peer[0], peer[1], peer[2], peer[3])),
                    u16::from_be_bytes([peer[4], peer[5]]),
                );
                peers.push(formatted_peer);
            }
            i += 6;
        }
        Ok(peers)
    }

    pub fn get_piece_hashes(&self) -> Vec<String> {
        let mut hashes = Vec::new();
        for i in 0..self.get_piece_count() {
            let hash = self.get_piece_hash(i);
            hashes.push(hex::encode(hash));
        }

        hashes
    }

    pub fn get_piece_count(&self) -> usize {
        self.info.pieces.len() / 20
    }

    pub fn get_piece_length(&self, piece_index: usize) -> u32 {
        if piece_index == self.get_piece_count() - 1 {
            self.info.length % self.info.piece_length
        } else {
            self.info.piece_length
        }
    }

    pub fn get_piece_hash(&self, piece_index: usize) -> Vec<u8> {
        self.info.pieces[piece_index * 20..(piece_index + 1) * 20].to_vec()
    }
}
