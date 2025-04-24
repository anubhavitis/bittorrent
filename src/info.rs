use std::{
    fs::File,
    io::{BufReader, Read},
};

use serde::Deserialize;
use serde_bencode::from_bytes;

#[derive(Debug, Deserialize)]
struct Torrent {
    announce: String,
    info: Info,
}

#[derive(Debug, Deserialize)]
struct Info {
    name: String,
    length: i64,
    pieces: Vec<u8>,
    piece_length: i64,
}

pub fn get_info(file_name: &str) -> serde_json::Value {
    let file = File::open(file_name).unwrap();
    let mut reader = BufReader::new(file);
    let mut file_bytes = Vec::new();
    reader.read_to_end(&mut file_bytes).unwrap();

    let torrent: Torrent = from_bytes(&file_bytes).unwrap();
    eprintln!("{:?}", torrent);
    eprintln!("Tracker URL: {}", torrent.announce);
    eprintln!("Info: {:?}", torrent.info);

    serde_json::json!({
        "Tracker URL": torrent.announce,
        "Length": torrent.info.length,
    })
}
