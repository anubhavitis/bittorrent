use clap::{Parser, Subcommand};
use std::{net::SocketAddr, path::PathBuf};

use codecrafters_bittorrent::handler;
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
        } => handler::download_piece_handler(save_path, torrent, piece_index).await,
    }
}
