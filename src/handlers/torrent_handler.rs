use serde_json::Number;
use std::{fs::File, io::Write, net::SocketAddr, path::PathBuf};

use crate::handshake::HandshakeMessage;
use crate::tcp::TcpManager;
use crate::torrent::{client::Client, torrent::Torrent};

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
    let torrent = Torrent::from(file_name);
    torrent.pretty_print();
}

pub async fn peers(file_name: &std::path::PathBuf) {
    let torrent = Torrent::from(file_name);
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
    let torrent = Torrent::from(&torrent);
    let handshake_message = HandshakeMessage::new(torrent.get_info_hash(), false);
    let mut stream = TcpManager::connect(peer).await;
    let handshake = stream
        .handshake(handshake_message)
        .await
        .expect("Failed to handshake");
    println!("Peer ID: {}", hex::encode(handshake.peer_id));
}

pub async fn download_piece(save_path: PathBuf, torrent: PathBuf, piece_index: u32) {
    let torrent = Torrent::from(&torrent);
    let peer = torrent.get_peers().await.unwrap()[0];
    let mut client = Client::new(torrent);
    client.handshake(peer).await.unwrap();
    client.init_download().await.unwrap();
    let data = client.download_piece(piece_index).await.unwrap();
    let mut file = File::create(save_path).unwrap();
    file.write_all(&data).unwrap();
}

pub async fn downlaod(save_path: PathBuf, torrent: PathBuf) {
    let torrent = Torrent::from(&torrent);
    let peer = torrent.get_peers().await.unwrap()[0];
    let mut client = Client::new(torrent.clone());
    client.handshake(peer).await.unwrap();
    client.init_download().await.unwrap();
    let pieces = torrent.get_piece_count();
    let mut data = Vec::new();
    for i in 0..pieces {
        let piece_data = client.download_piece(i as u32).await.unwrap();
        data.extend_from_slice(&piece_data);
    }
    let mut file = File::create(save_path).unwrap();
    file.write_all(&data).unwrap();
}
