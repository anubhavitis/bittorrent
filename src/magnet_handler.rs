use crate::magnet::magnet_link::MagnetLink;

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
    let peer_id = magnet.handshake().await.unwrap();
    println!("Peer ID: {:?}", hex::encode(peer_id));
}
