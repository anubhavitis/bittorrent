use std::net::SocketAddr;

use anyhow::Error;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::handshake::HandshakeMessage;

pub struct TcpManager {
    stream: TcpStream,
}

impl TcpManager {
    pub async fn connect(peer: SocketAddr) -> Self {
        let stream = TcpStream::connect(peer).await.unwrap();
        Self { stream }
    }

    pub async fn disconnect(&mut self) {
        self.stream.shutdown().await.unwrap();
    }

    pub async fn handshake(
        &mut self,
        handshake_message: HandshakeMessage,
    ) -> Result<[u8; 20], Error> {
        self.stream
            .write_all(&handshake_message.to_bytes())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to write handshake message: {}", e))?;

        let mut buffer = [0u8; 68];
        self.stream
            .read_exact(&mut buffer)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read handshake message: {}", e))?;

        let handshake_message = HandshakeMessage::from_bytes(&buffer);
        Ok(handshake_message.peer_id)
    }
}
