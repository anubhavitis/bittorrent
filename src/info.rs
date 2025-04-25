use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use sha1::{Digest, Sha1};

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

    let mut hasher = Sha1::new();
    let info_bytes = serde_bencode::to_bytes(&torrent.info).unwrap();
    hasher.update(&info_bytes);
    let hash = hasher.finalize();
    println!("Info Hash: {}", hex::encode(hash));

    println!("Piece Length: {}", torrent.info.piece_length);
    println!("Piece Hashes:");
    let hashes = get_piece_hashes(&torrent.info.pieces);
    for hash in hashes {
        println!("{}", hash);
    }
}

fn get_piece_hashes(pieces: &ByteBuf) -> Vec<String> {
    let mut hashes = Vec::new();
    for i in 0..pieces.len() / 20 {
        let hash = pieces[i * 20..(i + 1) * 20].to_vec();
        hashes.push(hex::encode(hash));
    }

    hashes
}
