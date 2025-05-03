use crate::handshake::HandshakeMessage;
use crate::peer_messages::{MessageId, PeerMessage, PiecePayload, RequestPayload};
use crate::torrent::Torrent;
use sha1::{Digest, Sha1};
use std::cmp::min;
use std::fs::File;
use std::io::{self, Write};
use std::net::SocketAddr;
use std::path::PathBuf;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Failed to connect to peer: {0}")]
    ConnectionFailed(String),

    #[error("Failed to read message: {0}")]
    ReadError(String),

    #[error("Failed to write message: {0}")]
    WriteError(String),

    #[error("Invalid message format: {0}")]
    InvalidMessage(String),

    #[error("File operation failed: {0}")]
    FileError(String),
}

type Result<T> = std::result::Result<T, ClientError>;

pub struct Client {
    torrent: Torrent,
    tcp_stream: TcpStream,
    fetched_data: Vec<u8>,
    current_piece_index: usize,
    piece_lengths: Vec<u32>,
    reading_length: u32,
    ready_to_request: bool,
}

impl Client {
    pub async fn new(torrent: Torrent, peer: SocketAddr) -> Result<Self> {
        let tcp_stream = TcpStream::connect(peer)
            .await
            .map_err(|e| ClientError::ConnectionFailed(e.to_string()))?;

        // Calculate piece lengths
        let mut piece_lengths = vec![];
        let mut remaining_length = torrent.info.length;

        while remaining_length > 0 {
            let piece_size = min(remaining_length, torrent.info.piece_length);
            piece_lengths.push(piece_size);
            remaining_length -= piece_size;
        }

        Ok(Self {
            torrent,
            tcp_stream,
            fetched_data: vec![],
            current_piece_index: 0,
            piece_lengths,
            reading_length: 1 << 14, // 16KB chunks
            ready_to_request: false,
        })
    }

    async fn read_message(&mut self) -> Result<Vec<u8>> {
        eprintln!("\n\nReading message");

        // Read message length (4 bytes)
        let mut length_buffer = [0u8; 4];
        self.tcp_stream
            .read_exact(&mut length_buffer)
            .await
            .map_err(|e| ClientError::ReadError(format!("Failed to read message length: {}", e)))?;

        let length = u32::from_be_bytes(length_buffer);

        // Handle keep-alive message (length = 0)
        if length == 0 {
            eprintln!("Received keep-alive message (length=0)");
            return Ok(vec![]);
        }

        eprintln!("Reading message of length: {}", length);

        // Read message content
        let mut message_buffer = vec![0u8; length as usize];
        self.tcp_stream
            .read_exact(&mut message_buffer)
            .await
            .map_err(|e| {
                ClientError::ReadError(format!("Failed to read message content: {}", e))
            })?;

        Ok(message_buffer)
    }

    pub async fn handshake(&mut self, handshake_message: HandshakeMessage) -> Result<String> {
        let handshake_bytes = handshake_message.to_bytes();

        // Send handshake
        self.tcp_stream
            .write_all(&handshake_bytes)
            .await
            .map_err(|e| ClientError::WriteError(format!("Failed to send handshake: {}", e)))?;

        // Read handshake response
        let mut buffer = [0u8; 68];
        self.tcp_stream.read_exact(&mut buffer).await.map_err(|e| {
            ClientError::ReadError(format!("Failed to read handshake response: {}", e))
        })?;

        let response = HandshakeMessage::from_bytes(&buffer);
        Ok(hex::encode(response.peer_id))
    }

    pub async fn send_cancel_message(&mut self) -> Result<()> {
        eprintln!("Sending Cancel message");
        let cancel_message = PeerMessage::new(MessageId::Cancel, vec![]);
        let cancel_bytes = cancel_message.to_bytes();

        self.tcp_stream
            .write_all(&cancel_bytes)
            .await
            .map_err(|e| {
                ClientError::WriteError(format!("Failed to send Cancel message: {}", e))
            })?;

        Ok(())
    }

    async fn send_interested_message(&mut self) -> Result<()> {
        eprintln!("Sending Interested message");
        let interested_message = PeerMessage::new(MessageId::Interested, vec![]);
        let interested_bytes = interested_message.to_bytes();

        self.tcp_stream
            .write_all(&interested_bytes)
            .await
            .map_err(|e| {
                ClientError::WriteError(format!("Failed to send Interested message: {}", e))
            })?;

        Ok(())
    }

    async fn send_request_message(
        &mut self,
        piece_index: usize,
        begin: u32,
        length: u32,
    ) -> Result<()> {
        dbg!(piece_index, begin, length);
        let payload = RequestPayload::new(piece_index as u32, begin, length);
        let request_message = PeerMessage::new(MessageId::Request, payload.to_bytes());
        let request_bytes = request_message.to_bytes();

        self.tcp_stream
            .write_all(&request_bytes)
            .await
            .map_err(|e| {
                ClientError::WriteError(format!("Failed to send Request message: {}", e))
            })?;

        Ok(())
    }

    pub fn cmp_piece_hash(&self) -> bool {
        let piece_hashes = self.torrent.get_piece_hashes();
        let piece_hash = &piece_hashes[self.current_piece_index];

        let mut hasher = Sha1::new();
        hasher.update(&self.fetched_data);
        let computed_hash = hasher.finalize();
        let computed_hash_str = hex::encode(computed_hash);

        piece_hash == &computed_hash_str
    }

    pub fn get_fetched_data(&self) -> &[u8] {
        &self.fetched_data
    }

    pub async fn create_file(&self, save_path: &PathBuf, data: &[u8]) -> Result<()> {
        let mut file = File::create(save_path)
            .map_err(|e| ClientError::FileError(format!("Failed to create file: {}", e)))?;

        file.write_all(data)
            .map_err(|e| ClientError::FileError(format!("Failed to write data: {}", e)))?;

        file.flush()
            .map_err(|e| ClientError::FileError(format!("Failed to flush file: {}", e)))?;

        Ok(())
    }

    async fn message_handler(&mut self) -> Result<()> {
        loop {
            let message_buffer = self.read_message().await?;

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
                    self.send_interested_message().await?;
                }
                MessageId::Request => {
                    eprintln!("Received Request message");
                }
                MessageId::Piece => {
                    // Process piece data
                    let fetched_piece = PeerMessage::from_bytes(&message_buffer);
                    let fetched_piece_payload = PiecePayload::from_bytes(&fetched_piece.payload);

                    // Append received block to our data
                    self.fetched_data
                        .extend_from_slice(&fetched_piece_payload.block);

                    // Calculate next request parameters
                    let next_begin =
                        fetched_piece_payload.begin + fetched_piece_payload.block.len() as u32;
                    let next_len = min(
                        self.reading_length,
                        self.piece_lengths[self.current_piece_index] - next_begin,
                    );

                    dbg!(next_begin, next_len);

                    // If no more data to fetch for this piece, return
                    if next_len == 0 {
                        eprintln!("No more data to fetch for this piece");
                        return Ok(());
                    }

                    // Request next block
                    self.send_request_message(self.current_piece_index, next_begin, next_len)
                        .await?;
                    eprintln!("Request message sent for next block");
                }
                MessageId::Cancel => {
                    eprintln!("Received Cancel message");
                }
            }
        }

        Ok(())
    }

    pub async fn download_piece(&mut self, piece_index: usize) -> Result<()> {
        // Handle initial message exchange if not ready
        if !self.ready_to_request {
            eprintln!("Executing pre-request handler");
            self.message_handler().await?;
            eprintln!("Pre-request handler done");
        }

        // Clear any previously fetched data
        self.fetched_data.clear();

        // eprintln!(
        //     "\n\n#############################\nDownloading piece: {}\n#############################\n\n",
        //     piece_index
        // );

        // Set current piece and calculate initial request size
        self.current_piece_index = piece_index;
        let reading_length = min(self.reading_length, self.piece_lengths[piece_index]);

        // Request first block of the piece
        self.send_request_message(piece_index, 0, reading_length)
            .await?;

        // Handle messages until piece is complete
        self.message_handler().await?;

        eprintln!(
            "Downloaded {} bytes for piece {}",
            self.fetched_data.len(),
            piece_index
        );

        Ok(())
    }
}
