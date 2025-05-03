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
    fetched_data: Vec<u8>,
    current_piece_index: usize,
    piece_length: Vec<u32>,
    reading_length: u32,
    ready_to_request: bool,
}

impl Client {
    pub async fn new(torrent: Torrent, peer: SocketAddr) -> Self {
        let handshake_message = HandshakeMessage::new(torrent.get_info_hash());
        let tcp_stream = TcpStream::connect(peer)
            .await
            .expect("Failed to connect to peer");

        let mut total_length = torrent.info.length;
        let mut piece_length = vec![];
        while total_length > 0 {
            piece_length.push(min(total_length, torrent.info.piece_length));
            total_length -= piece_length.last().unwrap();
        }

        eprintln!("Piece length: {:?}", piece_length);

        Self {
            torrent,
            handshake_message,
            tcp_stream,
            fetched_data: vec![],
            current_piece_index: 0,
            piece_length: piece_length,
            reading_length: 1 << 14,
            ready_to_request: false,
        }
    }

    pub async fn read_message(&mut self) -> Vec<u8> {
        eprintln!("\n\nReading message");
        let mut length_buffer = [0u8; 4];
        self.tcp_stream
            .read_exact(&mut length_buffer)
            .await
            .expect("Failed to read message length");
        let length = u32::from_be_bytes(length_buffer);

        if length == 0 {
            eprintln!("Message length is 0, length buffer: {:?}", length_buffer);
            return vec![];
        }

        eprintln!("length buffer: {:?}", length_buffer);
        eprintln!("Reading message of length: {}", length);
        let mut message_buffer = vec![0u8; length as usize];
        self.tcp_stream
            .read_exact(&mut message_buffer)
            .await
            .expect("Failed to read message");
        eprintln!("Message read: {:?}\n\n", message_buffer);
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
        eprintln!("Sending Cancel message");
        let cancel_message = PeerMessage::new(MessageId::Cancel, vec![]);
        let cancel_bytes = cancel_message.to_bytes();
        self.tcp_stream
            .write_all(&cancel_bytes)
            .await
            .expect("Failed to send Cancel message");
    }

    pub async fn send_interested_message(&mut self) {
        eprintln!("Sending Interested message");
        let interested_message = PeerMessage::new(MessageId::Interested, vec![]);
        let interested_bytes = interested_message.to_bytes();
        self.tcp_stream
            .write_all(&interested_bytes)
            .await
            .expect("Failed to send Interested message");
    }

    pub async fn send_request_message(&mut self, piece_index: usize, begin: u32, length: u32) {
        eprintln!(
            "Sending Request message for piece: {}, begin: {}, length: {}",
            piece_index, begin, length
        );
        let payload = RequestPayload::new(piece_index, begin, length);
        let request_message = PeerMessage::new(MessageId::Request, payload.to_bytes());
        eprintln!("Request message: {:?}", request_message);
        let request_bytes = request_message.to_bytes();
        eprintln!("Request bytes: {:?}", request_bytes);
        self.tcp_stream
            .write_all(&request_bytes)
            .await
            .expect("Failed to send Request message");
    }

    pub async fn cmp_piece_hash(&self) -> bool {
        let piece_hashes = self.torrent.get_piece_hashes();
        let piece_hash = piece_hashes[self.current_piece_index as usize].clone();
        let mut file_hash = Sha1::new();
        file_hash.update(self.fetched_data.as_slice());
        let file_hash = file_hash.finalize();
        let file_hash_str = hex::encode(file_hash);

        piece_hash == file_hash_str
    }

    pub fn get_fetched_data(&self) -> &[u8] {
        self.fetched_data.as_slice()
    }

    pub async fn create_file(&self, save_path: &PathBuf, data: &[u8]) {
        let mut file = File::create(save_path).expect("Failed to create file");
        file.write_all(data).expect("Failed to write file");
        file.flush().expect("Failed to flush file");
    }

    pub async fn message_handler(&mut self) {
        loop {
            let message_buffer = self.read_message().await;
            let message_id = message_buffer[0];
            let mut piece_length = self.piece_length[self.current_piece_index];
            match MessageId::from(message_id) {
                MessageId::Choke => eprintln!("Received Choke message"),
                MessageId::Unchoke => {
                    eprintln!("Received Unchoke message");
                    break;
                }
                MessageId::Interested => eprintln!("Received Interested message"),
                MessageId::NotInterested => eprintln!("Received NotInterested message"),
                MessageId::Have => eprintln!("Received Have message"),
                MessageId::Bitfield => {
                    eprintln!("Received Bitfield message");
                    self.send_interested_message().await;
                }
                MessageId::Request => eprintln!("Received Request message"),
                MessageId::Piece => {
                    let fetched_piece = PeerMessage::from_bytes(&message_buffer);
                    eprintln!("Received Piece message {:?}", fetched_piece);
                    let fetched_piece_payload = PiecePayload::from_bytes(&fetched_piece.payload);
                    self.fetched_data
                        .extend_from_slice(&fetched_piece_payload.block);
                    piece_length -= fetched_piece_payload.block.len() as u32;

                    if piece_length == 0 {
                        // No more data to fetch
                        eprintln!("No more data to fetch");
                        break;
                    }

                    let next_begin =
                        fetched_piece_payload.begin + fetched_piece_payload.block.len() as u32;
                    let next_len = min(2 << 13, piece_length);
                    self.send_request_message(self.current_piece_index, next_begin, next_len)
                        .await;

                    eprintln!("Request message sent");
                }
                MessageId::Cancel => eprintln!("Received Cancel message"),
            }
        }
    }

    pub async fn download_piece(&mut self, piece_index: usize) {
        if !self.ready_to_request {
            eprintln!("Executing pre-request handler");
            self.message_handler().await;
            eprintln!("Pre-request handler done");
        }

        eprintln!(
            "Total fetched {}, reading {} length",
            self.fetched_data.len(),
            self.piece_length[piece_index]
        );

        self.current_piece_index = piece_index;
        let reading_length = min(self.reading_length, self.piece_length[piece_index]);
        self.send_request_message(piece_index, 0, reading_length)
            .await;
        loop {
            self.message_handler().await;
        }
    }
}
