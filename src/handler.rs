use std::{net::SocketAddr, path::PathBuf};

use crate::{client::Client, handshake::HandshakeMessage, torrent::Torrent};
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
    let tracker_url = torrent.announce.clone();
    let hashes = torrent.get_piece_hashes();

    println!("Tracker URL: {}", tracker_url);
    println!("Length: {}", torrent.info.length);
    println!("Info Hash: {}", info_hash_str);
    println!("Piece Length: {}", torrent.info.piece_length);
    println!("Piece Hashes:");
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
    let mut client = Client::new(torrent);
    client.handshake(peer).await;
}

pub async fn download_piece_handler(save_path: PathBuf, torrent: PathBuf, piece_index: u32) {
    eprintln!("Downloading piece: {} {}", piece_index, save_path.display());
    let torrent = Torrent::new(&torrent);
    let peers = torrent.get_peers().await.expect("Failed to get peers");
    eprintln!("Peers: {:?}", peers);
    // let mut handles = vec![];
    // for peer in peers {
    //     let mut client = Client::new();
    //     let handshake_message = HandshakeMessage::new(torrent.get_info_hash());
    //     let handle = tokio::spawn(async move {
    //         client.handle_peer(handshake_message.clone(), peer).await;
    //     });
    //     handles.push(handle);
    // }

    // for handle in handles {
    //     handle.await.unwrap();
    // }

    let mut client = Client::new(torrent);
    client.handle_peer(peers[0], &save_path, piece_index).await;
}
