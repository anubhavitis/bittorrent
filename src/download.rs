use crate::torrent::Torrent;
use std::path::PathBuf;

pub fn download_piece(save_path: PathBuf, torrent: PathBuf, piece_index: u32) {
    println!("Downloading piece {} from {:?}", piece_index, torrent);
    let torrent_file = Torrent::new(&torrent);
    println!("{:?}", torrent_file);
    println!("{}", save_path.display());
}
