use std::{collections::HashMap, net::SocketAddr};

use anyhow::Error;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::peer_messages::{MessageId, PeerMessage};
use crate::{handshake::HandshakeMessage, peer_messages::ExtensionPayload};

#[derive(Debug)]
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
    ) -> Result<HandshakeMessage, Error> {
        let handshake_message_bytes = handshake_message.to_bytes();
        self.stream
            .write_all(&handshake_message_bytes)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send handshake message: {}", e))?;

        let mut buffer = [0; 68];
        self.stream
            .read_exact(&mut buffer)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read handshake response: {}", e))?;

        let resp = HandshakeMessage::from_bytes(&buffer);
        Ok(resp)
    }

    pub async fn extension_handshake(&mut self) -> Result<ExtensionPayload, Error> {
        let mut msg_bytes = vec![0u8];
        let msg = HashMap::from([(
            "m".to_string(),
            HashMap::from([("ut_metadata".to_string(), 21)]),
        )]);
        msg_bytes.extend(serde_bencode::to_bytes(&msg).unwrap());

        // sending extension handshake message
        self.send_message(MessageId::Extension, msg_bytes).await?;

        // reading extension handshake response
        let (_msg_id, payload) = self.read_message().await?;
        println!("message Id: {:?}", _msg_id);
        let extension_payload = ExtensionPayload::from_bytes(&payload);
        Ok(extension_payload)
    }

    pub async fn read_message(&mut self) -> Result<(MessageId, Vec<u8>), Error> {
        let mut length_buffer = [0u8; 4];
        self.stream
            .read_exact(&mut length_buffer)
            .await
            .expect("Failed to read message length");
        let length = u32::from_be_bytes(length_buffer);

        if length == 0 {
            return Err(anyhow::anyhow!("received empty message (length=0)"));
        }

        let mut message_buffer = vec![0u8; length as usize];
        self.stream
            .read_exact(&mut message_buffer)
            .await
            .expect("Failed to read message content");

        let message_id = MessageId::from(message_buffer[0]);
        let payload = message_buffer[1..].to_vec();

        Ok((message_id, payload))
    }

    pub async fn send_message(
        &mut self,
        message_id: MessageId,
        payload: Vec<u8>,
    ) -> Result<(), Error> {
        let message = PeerMessage::new(message_id, payload);
        let message_bytes = message.to_bytes();
        self.stream
            .write_all(&message_bytes)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send message: {}", e))?;

        Ok(())
    }
}
