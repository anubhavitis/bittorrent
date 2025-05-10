use bittorrent::handlers::command::Args;
use clap::Parser;

#[tokio::main]
async fn main() {
    let args = Args::parse();
    args.handle().await;
}
