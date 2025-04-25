use std::{env, path::PathBuf};

mod bencode;
mod info;
fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: your_bittorrent.sh decode <encoded_value>");
        return;
    }

    let command = &args[1];

    if command == "decode" {
        // You can use print statements as follows for debugging, they'll be visible when running tests.
        eprintln!("Logs from your program will appear here!");

        // Uncomment this block to pass the first stage
        let encoded_value = &args[2];
        let (decoded_value, _) = bencode::decode_bencoded_value(encoded_value);
        println!("{}", decoded_value.to_string());
    } else if command == "info" {
        let file_name = PathBuf::from(&args[2]);
        info::get_info(&file_name);
    } else {
        println!("unknown command: {}", args[1])
    }
}
