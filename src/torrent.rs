use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use sha1::{Digest, Sha1};

#[derive(Debug, Serialize, Deserialize)]
pub struct Torrent {
    pub announce: String,
    pub info: Info,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Info {
    pub name: String,
    pub length: i64,
    pub pieces: ByteBuf,
    #[serde(rename = "piece length")]
    pub piece_length: i64,
}

impl Torrent {
    pub fn new(file_name: &std::path::PathBuf) -> Self {
        let file = std::fs::read(file_name).expect("Failed to read the file");
        let torrent: Torrent = serde_bencode::from_bytes(&file).unwrap();
        torrent
    }

    pub fn get_info_hash(&self) -> [u8; 20] {
        let mut hasher = Sha1::new();
        let info_bytes = serde_bencode::to_bytes(&self.info).unwrap();
        hasher.update(&info_bytes);
        let hash = hasher.finalize();
        hash.try_into().expect("Failed to convert hash to array")
    }
}
