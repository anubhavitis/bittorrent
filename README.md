# Rust BitTorrent Client

A lightweight and efficient BitTorrent client implementation in Rust. This project demonstrates the core concepts of the BitTorrent protocol, including torrent file parsing, peer communication, and file downloading capabilities.

## Features

- Parse and validate .torrent files
- HTTP tracker communication
- Peer protocol implementation
- Efficient file downloading with pipelining
- Written in Rust for performance and safety
  
## Prerequisites

- Rust (latest stable version)
- Cargo package manager


## TEST

```
git clone https://github.com/anubhavitis/bittorrent.git
cd bittorrent
```

Download via magnet:
```
cargo run magnet_download -o test.gif  "magnet:?xt=urn:btih:c5fb9894bdaba464811b088d806bdd611ba490af&dn=magnet3.gif&tr=http%3A%2F%2Fbittorrent-test-tracker.codecrafters.io%2Fannounce"
``` 

Download via provided sample.torrent
```
cargo run download -o test2.txt sample.torrent
``` 
  
## Issues
currently all the peices are getting downloaded from the same peer. 
  - Will implement concurrent download using mulitple peers
  - And, as per bittorent paper, we should be downloading the rarest piece first. Will implement that soon too.


## Project Structure

- `src/main.rs` - Entry point of the application
- `src/torrent.rs` - Torrent file parsing and handling
- `src/peer.rs` - Peer protocol implementation
- `src/tracker.rs` - Tracker communication logic

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- The BitTorrent Protocol Specification
- The Rust community for their excellent documentation and tools
