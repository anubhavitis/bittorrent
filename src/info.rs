use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;

#[derive(Debug, Serialize, Deserialize)]
struct Torrent {
    announce: String,
    info: Info,
}

#[derive(Debug, Serialize, Deserialize)]
struct Info {
    name: String,
    length: i64,
    pieces: ByteBuf,
    #[serde(rename = "piece length")]
    piece_length: i64,
}

pub fn get_info(file_name: &std::path::PathBuf) {
    let file = std::fs::read(file_name).expect("Failed to read the file");
    let torrent: Torrent = serde_bencode::from_bytes(&file).unwrap();
    println!("Tracker URL: {}", torrent.announce);
    println!("Length: {}", torrent.info.length);
}
