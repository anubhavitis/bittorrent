use std::{fs::File, io::Write, path::PathBuf};

use crate::{
    magnet::magnet_link::MagnetLink,
    manager::{client::Client, torrent::Torrent},
};

pub fn parse(magnet_link: String) {
    let magnet_link = MagnetLink::from(magnet_link)
        .map_err(|e| e.to_string())
        .unwrap();
    println!("Info Hash: {}", magnet_link.info_hash);
    println!("Tracker URL: {}", magnet_link.tracker_url.unwrap());
}

pub async fn handshake(magnet_link: String) {
    let mut magnet = MagnetLink::from(magnet_link)
        .map_err(|e| e.to_string())
        .unwrap();
    let (peer_id, extension_id) = magnet.extension_handshake().await.unwrap();
    println!("Peer ID: {}", peer_id);
    println!("Peer Metadata Extension ID: {}", extension_id);
}

pub async fn fetch_metadata_info(magnet_link: String) {
    let mut magnet = MagnetLink::from(magnet_link)
        .map_err(|e| e.to_string())
        .unwrap();
    let info = magnet.fetch_metadata_info().await.unwrap();
    let torrent = Torrent::new(magnet.tracker_url.as_ref().unwrap().to_string(), info);
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

pub async fn download_piece(magnet_link: String, save_path: PathBuf, piece_index: u32) {
    let mut magnet = MagnetLink::from(magnet_link)
        .map_err(|e| e.to_string())
        .unwrap();
    let info = magnet.fetch_metadata_info().await.unwrap();
    let torrent = Torrent::new(magnet.tracker_url.as_ref().unwrap().to_string(), info);
    let mut client = Client::new(torrent);
    let peer = magnet.fetch_peers().await.unwrap()[0];
    client.handshake(peer).await.unwrap();
    client.init_download().await.unwrap();
    let data = client.download_piece(piece_index).await.unwrap();
    let mut file = File::create(save_path).unwrap();
    file.write_all(&data).unwrap();
    file.flush().unwrap();
}
