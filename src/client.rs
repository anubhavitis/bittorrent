use crate::handshake::HandshakeMessage;
use crate::peer_messages::{MessageId, PeerMessage, PiecePayload, RequestPayload};
use crate::torrent::Torrent;
use sha1::{Digest, Sha1};
use std::cmp::min;
use std::fs::File;
use std::io::Write;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub struct Client {
    torrent: Torrent,
    handshake_message: HandshakeMessage,
    tcp_stream: TcpStream,
}

impl Client {
    pub async fn new(torrent: Torrent, peer: SocketAddr) -> Self {
        let handshake_message = HandshakeMessage::new(torrent.get_info_hash());
        let tcp_stream = TcpStream::connect(peer)
            .await
            .expect("Failed to connect to peer");
        Self {
            torrent,
            handshake_message,
            tcp_stream,
        }
    }

    pub async fn read_message(&mut self) -> Vec<u8> {
        let mut length_buffer = [0u8; 4];
        self.tcp_stream
            .read_exact(&mut length_buffer)
            .await
            .expect("Failed to read message length");
        let length = u32::from_be_bytes(length_buffer);

        let mut message_buffer = vec![0u8; length as usize];
        self.tcp_stream
            .read_exact(&mut message_buffer)
            .await
            .expect("Failed to read message");
        message_buffer
    }

    pub async fn handshake(&mut self) -> String {
        let handshake_bytes = self.handshake_message.to_bytes();
        self.tcp_stream
            .write_all(&handshake_bytes)
            .await
            .expect("write handshake");
        let mut buffer = [0u8; 68];
        self.tcp_stream
            .read_exact(&mut buffer)
            .await
            .expect("read handshake");
        let response = HandshakeMessage::from_bytes(&buffer);
        hex::encode(response.peer_id)
    }

    pub async fn send_cancel_message(&mut self) {
        let cancel_message = PeerMessage::new(MessageId::Cancel, vec![]);
        let cancel_bytes = cancel_message.to_bytes();
        self.tcp_stream
            .write_all(&cancel_bytes)
            .await
            .expect("Failed to send Cancel message");
    }

    pub async fn send_interested_message(&mut self) {
        let interested_message = PeerMessage::new(MessageId::Interested, vec![]);
        let interested_bytes = interested_message.to_bytes();
        self.tcp_stream
            .write_all(&interested_bytes)
            .await
            .expect("Failed to send Interested message");
    }

    pub async fn send_request_message(&mut self, piece_index: u32, begin: u32, length: u32) {
        let payload = RequestPayload::new(piece_index, begin, length);
        let request_message = PeerMessage::new(MessageId::Request, payload.to_bytes());
        let request_bytes = request_message.to_bytes();
        self.tcp_stream
            .write_all(&request_bytes)
            .await
            .expect("Failed to send Request message");
    }

    pub async fn cmp_piece_hash(&self, piece_index: u32, data: &[u8]) -> bool {
        let piece_hashes = self.torrent.get_piece_hashes();
        let piece_hash = piece_hashes[piece_index as usize].clone();
        let mut file_hash = Sha1::new();
        file_hash.update(data);
        let file_hash = file_hash.finalize();
        let file_hash_str = hex::encode(file_hash);
        piece_hash == file_hash_str
    }

    pub async fn create_file(&self, save_path: &PathBuf, data: &[u8]) {
        let mut file = File::create(save_path).expect("Failed to create file");
        file.write_all(data).expect("Failed to write file");
        file.flush().expect("Failed to flush file");
    }

    pub async fn handle_peer(&mut self, save_path: &PathBuf, piece_index: u32) {
        // Establish connection to peer
        self.handshake().await;

        let size_remaining =
            self.torrent.info.length - (piece_index as i64 * self.torrent.info.piece_length);
        let mut piece_length = min(self.torrent.info.piece_length as u32, size_remaining as u32);
        let mut fetched_data = vec![];

        loop {
            let message_buffer = self.read_message().await;
            let message_id = message_buffer[0];

            match MessageId::from(message_id) {
                MessageId::Choke => eprintln!("Message received: Choke"),
                MessageId::Unchoke => self.send_request_message(piece_index, 0, 2 << 13).await,
                MessageId::Interested => eprintln!("Message received: Interested"),
                MessageId::NotInterested => eprintln!("Message received: Not Interested"),
                MessageId::Have => eprintln!("Message received: Have"),
                MessageId::Bitfield => self.send_interested_message().await,
                MessageId::Request => eprintln!("Message received: Request"),
                MessageId::Piece => {
                    let piece = PeerMessage::from_bytes(&message_buffer);
                    let piece = PiecePayload::from_bytes(&piece.payload);
                    fetched_data.extend_from_slice(&piece.block);
                    piece_length -= piece.block.len() as u32;

                    if piece_length == 0 {
                        // ###### Send Cancellation ######
                        self.send_cancel_message().await;
                        break;
                    } else {
                        // ###### REQUEST NEXT PIECE ######
                        let next_begin = piece.begin + piece.block.len() as u32;
                        let next_len = min(2 << 13, piece_length);
                        self.send_request_message(piece_index, next_begin, next_len)
                            .await;
                    }
                }
                MessageId::Cancel => eprintln!("Message received: Cancel"),
            }
        }

        if self.cmp_piece_hash(piece_index, &fetched_data).await {
            self.create_file(save_path, &fetched_data).await;
        } else {
            eprintln!("Piece {} is corrupted", piece_index);
        }
    }
}
