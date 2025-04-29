use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;

use crate::torrent::Torrent;

#[derive(Debug, Serialize, Deserialize)]
struct TrackerResponse {
    interval: i64,
    peers: ByteBuf,
}

pub fn get_info(file_name: &std::path::PathBuf) {
    let torrent = Torrent::new(file_name);
    let info_hash = torrent.get_info_hash();
    let info_hash_str = hex::encode(info_hash);
    let tracker_url = torrent.announce;

    println!("Tracker URL: {}", tracker_url);
    println!("Length: {}", torrent.info.length);
    println!("Info Hash: {}", info_hash_str);
    println!("Piece Length: {}", torrent.info.piece_length);
    println!("Piece Hashes:");
    let hashes = get_piece_hashes(&torrent.info.pieces);
    for hash in hashes {
        println!("{}", hash);
    }
}

pub async fn peers(file_name: &std::path::PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let torrent = Torrent::new(file_name);
    let info_hash = torrent.get_info_hash();
    let url_encoded_info_hash = urlencoding::encode_binary(&info_hash).to_string();

    let peer_id = generate_peer_id();
    let url_params = build_tracker_params(peer_id, torrent.info.length);
    let encoded_url_params = serde_urlencoded::to_string(&url_params)?;

    // Build the complete tracker URL
    let url = format!(
        "{}?{}&info_hash={}",
        torrent.announce, encoded_url_params, url_encoded_info_hash
    );

    // Send request to tracker and parse response
    let tracker_response = reqwest::get(url.as_str()).await?;
    let tracker_response_bytes = tracker_response.bytes().await?;
    let tracker_response: TrackerResponse = serde_bencode::from_bytes(&tracker_response_bytes)?;

    // Display peer information
    display_peers(&tracker_response.peers);

    Ok(())
}

fn generate_peer_id() -> String {
    "00112233445566778899".to_string()
}

fn build_tracker_params(peer_id: String, file_length: i64) -> serde_json::Value {
    serde_json::json!({
        "peer_id": peer_id,
        "port": 6881,
        "uploaded": 1,
        "downloaded": 1,
        "left": file_length,
        "compact": 1
    })
}

fn display_peers(peers_data: &ByteBuf) {
    let mut i = 0;
    while i < peers_data.len() {
        if i + 6 <= peers_data.len() {
            let peer = &peers_data[i..i + 6];
            let formatted_peer = formatter_peer(peer);
            println!("{}", formatted_peer);
        }
        i += 6;
    }
}

pub fn formatter_peer(peer: &[u8]) -> String {
    let ip = peer[..4]
        .iter()
        .map(|b| b.to_string())
        .collect::<Vec<String>>()
        .join(".");
    let port = u16::from_be_bytes(peer[4..6].try_into().unwrap());
    format!("{}:{}", ip, port)
}

fn get_piece_hashes(pieces: &ByteBuf) -> Vec<String> {
    let mut hashes = Vec::new();
    for i in 0..pieces.len() / 20 {
        let hash = pieces[i * 20..(i + 1) * 20].to_vec();
        hashes.push(hex::encode(hash));
    }

    hashes
}
