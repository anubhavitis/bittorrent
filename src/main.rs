use clap::{Parser, Subcommand};
use std::{net::SocketAddr, path::PathBuf};

use bittorrent::{handler, magnet_handler};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
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

#[tokio::main]
async fn main() {
    let args = Args::parse();
    match args.command {
        Command::Decode { encoded_value } => handler::decode_bencoded_value(encoded_value.as_str()),
        Command::Info { torrent } => handler::get_info(&torrent),
        Command::Peers { torrent } => handler::peers(&torrent).await,
        Command::Handshake { torrent, peer } => handler::handshake_handler(torrent, peer).await,
        Command::DownloadPiece {
            save_path,
            torrent,
            piece_index,
        } => handler::download_piece(save_path, torrent, piece_index).await,
        Command::Download { save_path, torrent } => handler::downlaod(save_path, torrent).await,
        Command::MagnetParse { link } => magnet_handler::parse(link),
        Command::MagnetHandshake { link } => magnet_handler::handshake(link).await,
        Command::MagnetInfo { link } => magnet_handler::fetch_metadata_info(link).await,
        Command::MagnetDownloadPiece {
            save_path,
            link,
            piece_index,
        } => magnet_handler::download_piece(link, save_path, piece_index).await,
        Command::MagnetDownload { save_path, link } => {
            magnet_handler::download_file(link, save_path).await
        }
    }
}
