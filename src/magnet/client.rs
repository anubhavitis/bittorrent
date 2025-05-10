use std::collections::HashMap;

use anyhow::Error;

use crate::{
    handshake::HandshakeMessage,
    magnet::magnet::MagnetLink,
    peer_messages::{ExtensionPayload, MessageId},
    tcp::TcpManager,
    torrent::torrent::Info,
};

pub struct MagnetClient {
    pub magnet: MagnetLink,
    client: TcpManager,
}

impl MagnetClient {
    pub async fn new(magnet: MagnetLink) -> Self {
        let peers = magnet.fetch_peers().await.unwrap();
        let client = TcpManager::connect(peers[0]).await;
        Self { magnet, client }
    }

    pub async fn extension_handshake(&mut self) -> Result<(String, u8), Error> {
        let handshake_message = HandshakeMessage::new(self.magnet.get_info_hash(), true);
        let handshake_resp = self.client.handshake(handshake_message).await?;

        if handshake_resp.reserved[5] != 16 {
            // this is mandatory for extension handshake
            return Err(anyhow::anyhow!(
                "Magnet handshake response has invalid reserved field"
            ));
        }

        let peer_id = hex::encode(handshake_resp.peer_id);

        let (msg_id, _payload) = self.client.read_message().await?;
        assert_eq!(msg_id, MessageId::Bitfield);

        let extension_handshake_payload = self.client.extension_handshake().await?;
        let extension_id = extension_handshake_payload.get_extension_id() as u8;

        Ok((peer_id, extension_id))
    }

    pub async fn fetch_metadata_info(&mut self, extension_id: u8) -> Result<Info, Error> {
        let msg_body = HashMap::from([("msg_type".to_string(), 0), ("piece".to_string(), 0)]);
        let mut msg = vec![extension_id];
        msg.extend(serde_bencode::to_bytes(&msg_body).unwrap());

        self.client.send_message(MessageId::Extension, msg).await?;

        let (msg_id, payload) = self.client.read_message().await?;
        assert_eq!(msg_id, MessageId::Extension);

        let (header, data) = split_header_and_data(&payload).unwrap();

        let extension_payload = ExtensionPayload::from_bytes(&header);
        assert_eq!(extension_payload.message_id, 21);

        let info = Info::from_bytes(&data);
        Ok(info)
    }
}

fn split_header_and_data(message: &[u8]) -> Option<(&[u8], &[u8])> {
    if let Some(pos) = message.windows(2).position(|window| window == b"ee") {
        Some((&message[..pos + 2], &message[pos + 2..]))
    } else {
        None
    }
}
