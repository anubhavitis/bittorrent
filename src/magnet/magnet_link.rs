use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use anyhow::Error;
use reqwest::Response;
use serde::{Deserialize, Serialize};
use serde_bencode::value::Value;
use serde_bytes::ByteBuf;

use crate::handshake::HandshakeMessage;
use crate::manager::torrent::{Info, Torrent};
use crate::peer_messages::ExtensionPayload;
use crate::{peer_messages::MessageId, tcp::TcpManager};

#[derive(Debug, Serialize, Deserialize)]
struct TrackerResponse {
    interval: u32,
    peers: ByteBuf,
}

#[derive(Debug)]
pub struct MagnetLink {
    pub info_hash: String,
    pub tracker_url: Option<String>,
    pub display_name: Option<String>,

    client: Option<TcpManager>,
}

impl MagnetLink {
    pub fn from(magnet_link: String) -> Result<Self, Error> {
        if !magnet_link.starts_with("magnet:?") {
            return Err(anyhow::anyhow!(
                "Invalid magnet link format: should start with 'magnet:?'"
            ));
        }

        let query = &magnet_link[8..];

        let mut result = MagnetLink {
            info_hash: String::new(),
            tracker_url: None,
            display_name: None,
            client: None,
        };

        for param in query.split('&') {
            let parts: Vec<&str> = param.splitn(2, '=').collect();
            if parts.len() != 2 {
                continue;
            }

            let (key, value) = (parts[0], parts[1]);
            match key {
                "xt" => {
                    if let Some(hash) = value.strip_prefix("urn:btih:") {
                        result.info_hash = hash.to_string();
                    } else if let Some(hash) = value.split(':').last() {
                        result.info_hash = hash.to_string();
                    }
                }
                "tr" => {
                    if let Ok(decoded) = urlencoding::decode(value) {
                        result.tracker_url = Some(decoded.into_owned());
                    }
                }
                "dn" => {
                    if let Ok(decoded) = urlencoding::decode(value) {
                        result.display_name = Some(decoded.into_owned());
                    } else {
                        result.display_name = Some(value.to_string());
                    }
                }
                _ => {} // Ignore other parameters
            }
        }

        Ok(result)
    }

    pub fn get_info_hash(&self) -> [u8; 20] {
        let info_hash_bytes: [u8; 20] = hex::decode(&self.info_hash).unwrap().try_into().unwrap();
        info_hash_bytes
    }

    async fn make_fetch_peer_request(&self) -> Result<Response, Error> {
        let params = [
            ("peer_id", "01012323454567678989"),
            ("port", "6881"),
            ("uploaded", "0"),
            ("downloaded", "0"),
            ("left", "999"),
            ("compact", "1"),
        ];

        let url = reqwest::Url::parse_with_params(self.tracker_url.as_ref().unwrap(), &params)
            .map_err(|e| anyhow::anyhow!("Failed to parse tracker URL: {}", e))?;

        let info_hash = self.get_info_hash();
        let encoded_info_hash = urlencoding::encode_binary(&info_hash).to_string();
        let url = format!("{url}&info_hash={encoded_info_hash}");

        let response = reqwest::get(url)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch peers: {}", e))?;

        Ok(response)
    }

    pub async fn fetch_peers(&self) -> Result<Vec<SocketAddr>, Error> {
        let response = self.make_fetch_peer_request().await?;
        let body_bytes = response.bytes().await.unwrap();
        let tracker_response: TrackerResponse = serde_bencode::from_bytes(&body_bytes)
            .map_err(|e| anyhow::anyhow!("Failed to parse tracker response: {}", e))?;
        let mut peers = Vec::new();
        for i in 0..tracker_response.peers.len() / 6 {
            let peer = tracker_response.peers[i * 6..(i + 1) * 6].to_vec();
            let ip = Ipv4Addr::new(peer[0], peer[1], peer[2], peer[3]);
            let port = u16::from_be_bytes([peer[4], peer[5]]);
            peers.push(SocketAddr::new(IpAddr::V4(ip), port));
        }

        Ok(peers)
    }

    async fn connect_client(&mut self, peer: SocketAddr) {
        let client = TcpManager::connect(peer).await;
        self.client = Some(client);
    }

    pub async fn extension_handshake(&mut self) -> Result<(String, u8), Error> {
        let peers = self.fetch_peers().await?;
        let handshake_message = HandshakeMessage::new(self.get_info_hash(), true);
        self.connect_client(peers[0]).await;

        let handshake_message = self
            .client
            .as_mut()
            .unwrap()
            .handshake(handshake_message)
            .await?;

        if handshake_message.reserved[5] != 16 {
            dbg!(&handshake_message.reserved);
            return Err(anyhow::anyhow!("Failed to get peer ID"));
        }

        let (_msg_id, _payload) = self.client.as_mut().unwrap().read_message().await?;

        let peer_id = hex::encode(handshake_message.peer_id);

        let msg = HashMap::from([(
            "m".to_string(),
            HashMap::from([("ut_metadata".to_string(), 21)]),
        )]);

        let mut msg_bytes = vec![0u8];
        msg_bytes.extend(serde_bencode::to_bytes(&msg).unwrap());

        self.client
            .as_mut()
            .unwrap()
            .send_message(MessageId::Extension, msg_bytes)
            .await?;

        let (_msg_id, payload) = self.client.as_mut().unwrap().read_message().await?;

        let extension_payload = ExtensionPayload::from_bytes(&payload);

        let extension_id = match &extension_payload.payload {
            Value::Dict(dict) => match dict.get(&b"m".to_vec()) {
                Some(Value::Dict(inner_dict)) => match inner_dict.get(&b"ut_metadata".to_vec()) {
                    Some(Value::Int(val)) => *val,
                    _ => 0,
                },
                _ => 0,
            },
            _ => 0,
        };

        Ok((peer_id, extension_id as u8))
    }

    pub async fn fetch_metadata_info(&mut self) -> Result<(), Error> {
        let (_, extension_id) = self.extension_handshake().await?;
        dbg!(&extension_id);
        let msg_body = HashMap::from([("msg_type".to_string(), 0), ("piece".to_string(), 0)]);
        let mut msg = vec![extension_id];
        msg.extend(serde_bencode::to_bytes(&msg_body).unwrap());
        self.client
            .as_mut()
            .unwrap()
            .send_message(MessageId::Extension, msg)
            .await?;

        let (msg_id, payload) = self.client.as_mut().unwrap().read_message().await?;
        assert_eq!(msg_id, MessageId::Extension);

        let (header, data) = split_header_and_data(&payload).unwrap();

        let extension_payload = ExtensionPayload::from_bytes(&header);
        assert_eq!(extension_payload.message_id, 21);

        let info = Info::from_bytes(&data);
        let torrent = Torrent::new(self.tracker_url.as_ref().unwrap().to_string(), info);
        let info_hash = torrent.get_info_hash();
        let info_hash_str = hex::encode(info_hash);
        let tracker_url = torrent.announce.clone();
        let hashes = torrent.get_piece_hashes();

        println!("Tracker URL: {}", tracker_url);
        println!("Length: {}", torrent.info.length);
        println!("Info Hash: {}", info_hash_str);
        println!("Piece Length: {}", torrent.info.piece_length);
        println!("Piece Hashes:");
        for hash in hashes {
            println!("{}", hash);
        }
        Ok(())
    }
}

fn split_header_and_data(message: &[u8]) -> Option<(&[u8], &[u8])> {
    if let Some(pos) = message.windows(2).position(|window| window == b"ee") {
        Some((&message[..pos + 2], &message[pos + 2..]))
    } else {
        None
    }
}
