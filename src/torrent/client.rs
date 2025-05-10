use std::{cmp::min, net::SocketAddr};

use anyhow::Error;
use sha1::{Digest, Sha1};

use crate::handshake::HandshakeMessage;
use crate::torrent::torrent::Torrent;
use crate::peer_messages::{MessageId, PiecePayload, RequestPayload};
use crate::tcp::TcpManager;
pub struct Client {
    torrent: Torrent,
    stream: Option<TcpManager>,
}

impl Client {
    pub fn new(torrent: Torrent) -> Self {
        Self {
            torrent,
            stream: None,
        }
    }

    pub fn set_stream(&mut self, stream: TcpManager) {
        self.stream = Some(stream);
    }

    pub async fn handshake(&mut self, peer: SocketAddr) -> Result<(), Error> {
        let stream = TcpManager::connect(peer).await;
        self.stream = Some(stream);

        let handshake_message = HandshakeMessage::new(self.torrent.get_info_hash(), false);
        let _ = self
            .stream
            .as_mut()
            .unwrap()
            .handshake(handshake_message)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to handshake: {}", e))?;

        // Read bitfield message
        let (message_id, _) = self
            .stream
            .as_mut()
            .unwrap()
            .read_message()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read bitfield message: {}", e))?;

        if message_id != MessageId::Bitfield {
            return Err(anyhow::anyhow!(
                "Expected bitfield message, got {:?}",
                message_id
            ));
        }

        Ok(())
    }

    pub async fn init_download(&mut self) -> Result<(), Error> {
        // Send interested message
        let _ = self
            .stream
            .as_mut()
            .unwrap()
            .send_message(MessageId::Interested, vec![])
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send interested message: {}", e))?;

        // Read unchoke message
        let (message_id, _) = self
            .stream
            .as_mut()
            .unwrap()
            .read_message()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read unchoke message: {}", e))?;

        if message_id != MessageId::Unchoke {
            return Err(anyhow::anyhow!(
                "Expected unchoke message, got {:?}",
                message_id
            ));
        }

        Ok(())
    }

    pub async fn download_piece(&mut self, piece_index: u32) -> Result<Vec<u8>, Error> {
        if self.stream.is_none() {
            return Err(anyhow::anyhow!("Stream is not initialized"));
        }

        let stream = self.stream.as_mut().unwrap();
        let piece_length = self.torrent.get_piece_length(piece_index as usize);
        let reading_len = 1 << 14;

        let mut data = Vec::new();
        let mut begin = 0u32;
        let mut length = min(reading_len as u32, piece_length - begin);

        while begin < piece_length {
            let request_message = RequestPayload::new(piece_index, begin, length);
            let _ = stream
                .send_message(MessageId::Request, request_message.to_bytes())
                .await
                .map_err(|e| anyhow::anyhow!("Failed to send request message: {}", e))?;

            let (message_id, payload) = stream.read_message().await?;
            if message_id != MessageId::Piece {
                return Err(anyhow::anyhow!(
                    "Expected piece message, got {:?}",
                    message_id
                ));
            }

            let piece_payload = PiecePayload::from_bytes(&payload);
            data.extend_from_slice(&piece_payload.block);

            begin += length;
            length = min(reading_len as u32, piece_length - begin);
        }

        if !self.cmp_hash(piece_index, data.clone()) {
            return Err(anyhow::anyhow!("corrupted piece downloaded"));
        }

        Ok(data)
    }

    fn cmp_hash(&self, piece_index: u32, data: Vec<u8>) -> bool {
        let piece_hash = self.torrent.get_piece_hash(piece_index as usize);
        let mut hasher = Sha1::new();
        hasher.update(data);
        let hash = hasher.finalize().to_vec();
        hash == piece_hash
    }
}
