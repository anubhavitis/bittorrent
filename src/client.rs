use crate::handshake::HandshakeMessage;
use crate::peer_messages::{MessageId, PeerMessage};
use crate::torrent::Torrent;
use sha1::{Digest, Sha1};
use std::cmp::{max, min};
use std::fs::File;
use std::io::Write;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub struct Client {
    torrent: Torrent,
    handshake_message: HandshakeMessage,
}

impl Client {
    pub fn new(torrent: Torrent) -> Self {
        let handshake_message = HandshakeMessage::new(torrent.get_info_hash());
        Self {
            torrent,
            handshake_message,
        }
    }

    pub async fn handshake(&mut self, peer: SocketAddr) {
        let mut tcp_stream = match TcpStream::connect(peer).await {
            Ok(stream) => stream,
            Err(e) => panic!("Failed to connect to peer: {}", e),
        };

        let handshake_bytes = self.handshake_message.to_bytes();
        tcp_stream
            .write_all(&handshake_bytes)
            .await
            .expect("write handshake");

        let mut buffer = [0u8; 68];
        tcp_stream
            .read_exact(&mut buffer)
            .await
            .expect("Failed to read handshake");
        let response = HandshakeMessage::from_bytes(&buffer);
        println!("Peer ID: {}", hex::encode(response.peer_id));
    }

    pub async fn handle_peer(&mut self, peer: SocketAddr, save_path: &PathBuf, piece_index: u32) {
        // Establish connection to peer
        eprintln!("Connecting to peer: {}", peer);
        let mut tcp_stream = TcpStream::connect(peer)
            .await
            .expect("Failed to connect to peer");

        // Send handshake to peer
        let handshake_bytes = self.handshake_message.to_bytes();
        tcp_stream
            .write_all(&handshake_bytes)
            .await
            .expect("write handshake");

        let mut buffer = [0u8; 68];
        tcp_stream
            .read_exact(&mut buffer)
            .await
            .expect("Failed to read handshake");
        let response = HandshakeMessage::from_bytes(&buffer);
        eprintln!(
            "Handshke completed with peer ID: {}",
            hex::encode(response.peer_id)
        );

        let size_remaining =
            self.torrent.info.length - (piece_index as i64 * self.torrent.info.piece_length);
        eprintln!(
            "Torrent length: {}, Piece index: {}, Piece length: {}",
            self.torrent.info.length, piece_index, self.torrent.info.piece_length
        );
        eprintln!("Size remaining: {}", size_remaining);
        let mut piece_length = min(self.torrent.info.piece_length as u32, size_remaining as u32);
        eprintln!("Piece length: {}", piece_length);
        let mut new_file = vec![];
        // Read messages in a loop
        loop {
            let mut length_buffer = [0u8; 4];
            tcp_stream
                .read_exact(&mut length_buffer)
                .await
                .expect("Failed to read message length");
            let length = u32::from_be_bytes(length_buffer);

            let mut message_buffer = vec![0u8; length as usize];
            tcp_stream
                .read_exact(&mut message_buffer)
                .await
                .expect("Failed to read message");

            let message_id = message_buffer[0];
            match MessageId::from(message_id) {
                MessageId::Choke => {
                    eprintln!("Message received: Choke");
                }
                MessageId::Unchoke => {
                    eprintln!("Message received: Unchoke");
                    let mut payload = vec![];
                    payload.extend_from_slice(&piece_index.to_be_bytes());
                    payload.extend_from_slice(&(0 as u32).to_be_bytes());
                    payload.extend_from_slice(&((2 << 13) as u32).to_be_bytes());
                    eprintln!("Payload Bytes: {:?}", payload);
                    let request_message = PeerMessage::new(MessageId::Request, payload);
                    let piece_bytes = request_message.to_bytes();
                    tcp_stream
                        .write_all(&piece_bytes)
                        .await
                        .expect("Failed to send Piece message");

                    eprintln!("Request message sent: {:?}", piece_bytes);
                }
                MessageId::Interested => {
                    eprintln!("Message received: Interested");
                }
                MessageId::NotInterested => {
                    eprintln!("Message received: Not Interested");
                }
                MessageId::Have => {
                    eprintln!("Message received: Have");
                }
                MessageId::Bitfield => {
                    eprintln!("Message received: Bitfield");
                    let new_message = PeerMessage::new(MessageId::Interested, vec![]);
                    let interested_bytes = new_message.to_bytes();
                    tcp_stream
                        .write_all(&interested_bytes)
                        .await
                        .expect("Failed to send Interested message");

                    eprintln!("Intersted sent to peer");
                }
                MessageId::Request => {
                    eprintln!("Message received: Request");
                }
                MessageId::Piece => {
                    eprintln!("Message received: Piece");
                    let piece = PeerMessage::from_bytes(&message_buffer);
                    eprintln!("Piece messageId: {:?}", piece.message_id);
                    let piece = Piece::from_bytes(&piece.payload);
                    eprintln!("Piece index: {:?}, begin: {:?}", piece.index, piece.begin);
                    new_file.extend_from_slice(&piece.block);
                    eprintln!("Successfully wrote {} bytes to file", piece.block.len());
                    piece_length -= piece.block.len() as u32;
                    eprintln!(" Remaining Piece length: {}", piece_length);

                    // ###### Check Cancellation ######
                    if piece_length == 0 {
                        eprintln!("Piece is complete, sending cancel message");
                        let cancel_message = PeerMessage::new(MessageId::Cancel, vec![]);
                        let cancel_bytes = cancel_message.to_bytes();
                        tcp_stream
                            .write_all(&cancel_bytes)
                            .await
                            .expect("Failed to send Cancel message");

                        break;
                    }
                    // ###### REQUEST NEXT PIECE ######

                    let next_begin = piece.begin + piece.block.len() as u32;
                    let next_len = min(2 << 13, piece_length);
                    let mut payload = vec![];
                    payload.extend_from_slice(&piece.index.to_be_bytes());
                    payload.extend_from_slice(&next_begin.to_be_bytes());
                    payload.extend_from_slice(&next_len.to_be_bytes());
                    let request_message = PeerMessage::new(MessageId::Request, payload);
                    let request_bytes = request_message.to_bytes();
                    tcp_stream
                        .write_all(&request_bytes)
                        .await
                        .expect("Failed to send Request message");

                    eprintln!("Request message sent: {:?}", request_bytes);
                }
                MessageId::Cancel => {
                    eprintln!("Message received: Cancel");
                }
            }
        }

        eprintln!("Piece {} is complete", piece_index);

        let piece_hashes = self.torrent.get_piece_hashes();
        let piece_hash = piece_hashes[piece_index as usize].clone();
        println!("Piece Hash: {}", piece_hash);

        let mut file_hash = Sha1::new();
        file_hash.update(&new_file);
        let file_hash = file_hash.finalize();
        let file_hash_str = hex::encode(file_hash);
        println!("File Hash: {}", file_hash_str);

        if piece_hash == file_hash_str {
            eprintln!("Piece {} is complete", piece_index);
            let mut file = File::create(save_path).expect("Failed to create file");
            file.write_all(&new_file).expect("Failed to write file");
            file.flush().expect("Failed to flush file");
        } else {
            eprintln!("Piece {} is corrupted", piece_index);
        }
    }
}

#[derive(Debug)]
struct Piece {
    index: u32,
    begin: u32,
    block: Vec<u8>,
}

impl Piece {
    fn from_bytes(bytes: &[u8]) -> Self {
        let index = u32::from_be_bytes(bytes[0..4].try_into().expect("Failed to convert index"));
        let begin = u32::from_be_bytes(bytes[4..8].try_into().expect("Failed to convert begin"));
        let block = bytes[8..].to_vec();
        Self {
            index,
            begin,
            block,
        }
    }
}
