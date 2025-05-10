use std::{net::SocketAddr, path::PathBuf};

use clap::{Parser, Subcommand};

use super::{magnet_handler, torrent_handler};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Decode {
        encoded_value: String,
    },
    Info {
        torrent: PathBuf,
    },
    Peers {
        torrent: PathBuf,
    },
    Handshake {
        torrent: PathBuf,
        peer: SocketAddr,
    },
    #[command(name = "download_piece")]
    DownloadPiece {
        #[arg(short = 'o')]
        save_path: PathBuf,
        torrent: PathBuf,
        piece_index: u32,
    },
    Download {
        #[arg(short = 'o')]
        save_path: PathBuf,
        torrent: PathBuf,
    },
    #[command(name = "magnet_parse")]
    MagnetParse {
        link: String,
    },
    #[command(name = "magnet_handshake")]
    MagnetHandshake {
        link: String,
    },
    #[command(name = "magnet_info")]
    MagnetInfo {
        link: String,
    },
    #[command(name = "magnet_download_piece")]
    MagnetDownloadPiece {
        #[arg(short = 'o')]
        save_path: PathBuf,
        link: String,
        piece_index: u32,
    },
    #[command(name = "magnet_download")]
    MagnetDownload {
        #[arg(short = 'o')]
        save_path: PathBuf,
        link: String,
    },
}

impl Args {
    pub async fn handle(&self) {
        match &self.command {
            Command::Decode { encoded_value } => {
                torrent_handler::decode_bencoded_value(encoded_value.as_str())
            }
            Command::Info { torrent } => torrent_handler::get_info(&torrent),
            Command::Peers { torrent } => torrent_handler::peers(&torrent).await,
            Command::Handshake { torrent, peer } => {
                torrent_handler::handshake_handler(torrent.clone(), peer.clone()).await
            }
            Command::DownloadPiece {
                save_path,
                torrent,
                piece_index,
            } => {
                torrent_handler::download_piece(save_path.clone(), torrent.clone(), *piece_index)
                    .await
            }
            Command::Download { save_path, torrent } => {
                torrent_handler::downlaod(save_path.clone(), torrent.clone()).await
            }
            Command::MagnetParse { link } => magnet_handler::parse(link.clone()),
            Command::MagnetHandshake { link } => magnet_handler::handshake(link.clone()).await,
            Command::MagnetInfo { link } => magnet_handler::fetch_metadata_info(link.clone()).await,
            Command::MagnetDownloadPiece {
                save_path,
                link,
                piece_index,
            } => {
                magnet_handler::download_piece(link.clone(), save_path.clone(), *piece_index).await
            }
            Command::MagnetDownload { save_path, link } => {
                magnet_handler::download_file(link.clone(), save_path.clone()).await
            }
        }
    }
}
