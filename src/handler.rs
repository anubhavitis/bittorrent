use std::{net::SocketAddr, path::PathBuf};

use crate::{download::download_piece, handshake::handshake, torrent::Torrent};
use serde_bytes::ByteBuf;
use serde_json::Number;

fn jsonify(value: &serde_bencode::value::Value) -> serde_json::Value {
    match value {
        serde_bencode::value::Value::Bytes(s) => {
            serde_json::Value::String(String::from_utf8(s.clone()).unwrap())
        }
        serde_bencode::value::Value::Int(i) => serde_json::Value::Number(Number::from(*i)),
        serde_bencode::value::Value::List(l) => {
            serde_json::Value::Array(l.iter().map(jsonify).collect())
        }
        serde_bencode::value::Value::Dict(d) => serde_json::Value::Object(
            d.iter()
                .map(|(k, v)| (String::from_utf8(k.clone()).unwrap(), jsonify(v)))
                .collect(),
        ),
    }
}

pub fn decode_bencoded_value(encoded_value: &str) {
    let value = serde_bencode::from_str(&encoded_value).unwrap();
    println!("{}", jsonify(&value));
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

pub async fn peers(file_name: &std::path::PathBuf) {
    let torrent = Torrent::new(file_name);
    let peers = torrent.get_peers().await;
    match peers {
        Ok(peers) => {
            for peer in peers {
                println!("{}", peer);
            }
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}

pub async fn handshake_handler(torrent: PathBuf, peer: SocketAddr) {
    let torrent = Torrent::new(&torrent);
    let info_hash = torrent.get_info_hash();
    handshake(info_hash, peer).await;
}

pub async fn download_piece_handler(save_path: PathBuf, torrent: PathBuf, piece_index: u32) {
    download_piece(save_path, torrent, piece_index);
}

fn get_piece_hashes(pieces: &ByteBuf) -> Vec<String> {
    let mut hashes = Vec::new();
    for i in 0..pieces.len() / 20 {
        let hash = pieces[i * 20..(i + 1) * 20].to_vec();
        hashes.push(hex::encode(hash));
    }

    hashes
}
