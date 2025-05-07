use crate::magnet::magnet_link::MagnetLink;

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
    magnet.fetch_metadata_info().await.unwrap();
}
