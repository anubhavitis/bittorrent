use std::{fs::File, io::Write, path::PathBuf};

use crate::{
    magnet::{client::MagnetClient, magnet::MagnetLink},
    torrent::{client::Client, torrent::Torrent},
};

pub fn parse(magnet_link: String) {
    let magnet_link = MagnetLink::from(magnet_link)
        .map_err(|e| e.to_string())
        .unwrap();
    println!("Info Hash: {}", magnet_link.info_hash);
    println!("Tracker URL: {}", magnet_link.tracker_url.unwrap());
}

pub async fn handshake(magnet_link: String) {
    let magnet = MagnetLink::from(magnet_link)
        .map_err(|e| e.to_string())
        .unwrap();
    let mut client = MagnetClient::new(magnet.clone()).await;
    let (peer_id, extension_id) = client.extension_handshake().await.unwrap();
    println!("Peer ID: {}", peer_id);
    println!("Peer Metadata Extension ID: {}", extension_id);
}

pub async fn fetch_metadata_info(magnet_link: String) {
    let magnet = MagnetLink::from(magnet_link)
        .map_err(|e| e.to_string())
        .unwrap();
    let mut client = MagnetClient::new(magnet.clone()).await;
    let (_peer_id, extension_id) = client.extension_handshake().await.unwrap();

    let info = client.fetch_metadata_info(extension_id).await.unwrap();
    let torrent = Torrent::new(magnet.tracker_url.as_ref().unwrap().to_string(), info);
    torrent.pretty_print();
}

pub async fn download_piece(magnet_link: String, save_path: PathBuf, piece_index: u32) {
    let magnet = MagnetLink::from(magnet_link)
        .map_err(|e| e.to_string())
        .unwrap();
    let mut client = MagnetClient::new(magnet.clone()).await;
    let (_peer_id, extension_id) = client.extension_handshake().await.unwrap();

    let info = client.fetch_metadata_info(extension_id).await.unwrap();
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

pub async fn download_file(magnet_link: String, save_path: PathBuf) {
    let magnet = MagnetLink::from(magnet_link)
        .map_err(|e| e.to_string())
        .unwrap();
    let mut client = MagnetClient::new(magnet.clone()).await;
    let (_peer_id, extension_id) = client.extension_handshake().await.unwrap();

    let info = client.fetch_metadata_info(extension_id).await.unwrap();
    let torrent = Torrent::new(magnet.tracker_url.as_ref().unwrap().to_string(), info);
    let mut client = Client::new(torrent.clone());
    let peer = magnet.fetch_peers().await.unwrap()[0];

    client.handshake(peer).await.unwrap();
    client.init_download().await.unwrap();
    let pieces = torrent.get_piece_count();
    let mut file = File::create(save_path).unwrap();
    for i in 0..pieces {
        let data = client.download_piece(i as u32).await.unwrap();
        file.write_all(&data).unwrap();
    }
    file.flush().unwrap();
}
