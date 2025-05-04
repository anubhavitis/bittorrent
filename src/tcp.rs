use crate::handshake::HandshakeMessage;
use crate::peer_messages::{MessageId, PeerMessage, PiecePayload, RequestPayload};
use sha1::{Digest, Sha1};
use std::cmp::min;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub struct TcpClient {
    stream: TcpStream,
    ready_to_request: bool,
    fetched_data: Vec<u8>,
    piece_index: usize,
    piece_length: u32,
}

impl TcpClient {
    pub async fn new(addr: SocketAddr) -> Self {
        let client = Self {
            stream: TcpStream::connect(addr)
                .await
                .expect("Failed to connect to peer"),
            ready_to_request: false,
            fetched_data: vec![],
            piece_index: 0,
            piece_length: 0,
        };

        client
    }

    pub async fn handshake(&mut self, handshake_message: HandshakeMessage) -> String {
        let handshake_bytes = handshake_message.to_bytes();

        // Send handshake
        self.stream
            .write_all(&handshake_bytes)
            .await
            .expect("Failed to send handshake");

        // Read handshake response
        let mut buffer = [0u8; 68];
        self.stream
            .read_exact(&mut buffer)
            .await
            .expect("Failed to read handshake response");

        let response = HandshakeMessage::from_bytes(&buffer);
        hex::encode(response.peer_id)
    }

    pub async fn send_cancel_message(&mut self) {
        // eprintln!("Sending Cancel message");
        let cancel_message = PeerMessage::new(MessageId::Cancel, vec![]);
        let cancel_bytes = cancel_message.to_bytes();

        self.stream
            .write_all(&cancel_bytes)
            .await
            .expect("Failed to send Cancel message");
    }

    async fn send_interested_message(&mut self) {
        // eprintln!("Sending Interested message");
        let interested_message = PeerMessage::new(MessageId::Interested, vec![]);
        let interested_bytes = interested_message.to_bytes();

        self.stream
            .write_all(&interested_bytes)
            .await
            .expect("Failed to send Interested message");
    }

    pub async fn read_message(&mut self) -> Vec<u8> {
        // eprintln!("\n\nReading message");

        // Read message length (4 bytes)
        let mut length_buffer = [0u8; 4];
        self.stream
            .read_exact(&mut length_buffer)
            .await
            .expect("Failed to read message length");

        let length = u32::from_be_bytes(length_buffer);

        // Handle keep-alive message (length = 0)
        if length == 0 {
            // eprintln!("Received keep-alive message (length=0)");
            return vec![];
        }

        // eprintln!("Reading message of length: {}", length);

        // Read message content
        let mut message_buffer = vec![0u8; length as usize];
        self.stream
            .read_exact(&mut message_buffer)
            .await
            .expect("Failed to read message content");

        message_buffer
    }

    async fn send_request_message(&mut self, piece_index: usize, begin: u32, length: u32) {
        eprintln!(
            "Sending Request message for piece {} of len {}",
            piece_index, length
        );
        let payload = RequestPayload::new(piece_index as u32, begin, length);
        let request_message = PeerMessage::new(MessageId::Request, payload.to_bytes());
        let request_bytes = request_message.to_bytes();

        self.stream
            .write_all(&request_bytes)
            .await
            .expect("Failed to send Request message");
    }

    async fn message_handler(&mut self) {
        loop {
            let message_buffer = self.read_message().await;

            // Handle keep-alive message
            if message_buffer.is_empty() {
                continue;
            }

            let message_id = message_buffer[0];
            match MessageId::from(message_id) {
                MessageId::Choke => {
                    eprintln!("Received Choke message");
                }
                MessageId::Unchoke => {
                    eprintln!("Received Unchoke message");
                    self.ready_to_request = true;
                    break;
                }
                MessageId::Interested => {
                    eprintln!("Received Interested message");
                }
                MessageId::NotInterested => {
                    eprintln!("Received NotInterested message");
                }
                MessageId::Have => {
                    eprintln!("Received Have message");
                }
                MessageId::Bitfield => {
                    eprintln!("Received Bitfield message");
                    self.send_interested_message().await;
                }
                MessageId::Request => {
                    eprintln!("Received Request message");
                }
                MessageId::Piece => {
                    // Process piece data
                    eprintln!("Received Piece message for index {}", self.piece_index);
                    let fetched_piece = PeerMessage::from_bytes(&message_buffer);
                    let fetched_piece_payload = PiecePayload::from_bytes(&fetched_piece.payload);

                    // Append received block to our data
                    self.fetched_data
                        .extend_from_slice(&fetched_piece_payload.block);

                    // Calculate next request parameters
                    let next_begin =
                        fetched_piece_payload.begin + fetched_piece_payload.block.len() as u32;
                    let next_len = min(1 << 14, self.piece_length - next_begin);

                    // dbg!(next_begin, next_len);

                    // If no more data to fetch for this piece, return
                    if next_len == 0 {
                        // eprintln!("No more data to fetch for this piece");
                        return;
                    }

                    // Request next block
                    self.send_request_message(self.piece_index, next_begin, next_len)
                        .await;
                    // eprintln!("Request message sent for next block");
                }
                MessageId::Cancel => {
                    eprintln!("Received Cancel message");
                }
            }
        }
    }

    pub async fn download_piece(
        &mut self,
        piece_index: usize,
        piece_length: u32,
        hash: String,
    ) -> Result<Vec<u8>, String> {
        // Handle initial message exchange if not ready
        if !self.ready_to_request {
            eprintln!("Executing pre-request handler");
            self.message_handler().await;
            eprintln!("Pre-request handler done");
        }

        self.fetched_data.clear();

        self.piece_index = piece_index;
        self.piece_length = piece_length;
        let reading_length = min(1 << 14, self.piece_length);

        // Request first block of the piece
        self.send_request_message(piece_index, 0, reading_length)
            .await;

        // Handle messages until piece is complete
        self.message_handler().await;

        eprintln!(
            "Downloaded {} bytes for piece {}",
            self.fetched_data.len(),
            piece_index
        );

        let mut hasher = Sha1::new();
        hasher.update(&self.fetched_data);
        let computed_hash = hasher.finalize();
        let computed_hash_str = hex::encode(computed_hash);

        if computed_hash_str == hash {
            Ok(self.fetched_data.clone())
        } else {
            eprintln!("Hash mismatch for piece {}", piece_index);
            Err(format!("Hash mismatch for piece {}", piece_index))
        }
    }
}
