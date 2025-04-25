use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod bencode;
mod info;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Decode { encoded_value: String },
    Info { file_name: PathBuf },
    Peers { file_name: PathBuf },
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    match args.command {
        Command::Decode { encoded_value } => {
            let (decoded_value, _) = bencode::decode_bencoded_value(encoded_value.as_str());
            println!("{}", decoded_value.to_string());
        }
        Command::Info { file_name } => info::get_info(&file_name),
        Command::Peers { file_name } => {
            let _ = info::peers(&file_name).await;
        }
    }
}
