use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct PeerMessage {
    pub length: [u8; 4],
    pub message_id: [u8; 1],
    pub payload: Vec<u8>,
}

#[derive(Debug, PartialEq)]
#[repr(u8)]
pub enum MessageId {
    Choke = 0,
    Unchoke = 1,
    Interested = 2,
    NotInterested = 3,
    Have = 4,
    Bitfield = 5,
    Request = 6,
    Piece = 7,
    Cancel = 8,
    Extension = 20,
}

impl From<u8> for MessageId {
    fn from(value: u8) -> Self {
        match value {
            0 => MessageId::Choke,
            1 => MessageId::Unchoke,
            2 => MessageId::Interested,
            3 => MessageId::NotInterested,
            4 => MessageId::Have,
            5 => MessageId::Bitfield,
            6 => MessageId::Request,
            7 => MessageId::Piece,
            8 => MessageId::Cancel,
            20 => MessageId::Extension,
            _ => panic!("Invalid message ID: {}", value),
        }
    }
}

impl PeerMessage {
    pub fn new(message_id: MessageId, payload: Vec<u8>) -> Self {
        let message_id = [message_id as u8];
        let length: [u8; 4] = ((message_id.len() as u32) + (payload.len() as u32)).to_be_bytes();

        Self {
            length,
            message_id,
            payload,
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let length = (bytes.len() as u32).to_be_bytes();
        let message_id = bytes[0..1].try_into().unwrap();
        let payload = bytes[1..].to_vec();
        Self {
            length,
            message_id,
            payload,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.length);
        bytes.extend_from_slice(&self.message_id);
        bytes.extend_from_slice(&self.payload);
        bytes
    }
}

#[derive(Debug)]
pub struct PiecePayload {
    pub index: u32,
    pub begin: u32,
    pub block: Vec<u8>,
}

impl PiecePayload {
    pub fn from_bytes(bytes: &[u8]) -> Self {
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

#[derive(Debug)]
pub struct RequestPayload {
    pub index: u32,
    pub begin: u32,
    pub length: u32,
}

impl RequestPayload {
    pub fn new(index: u32, begin: u32, length: u32) -> Self {
        Self {
            index,
            begin,
            length,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];
        bytes.extend_from_slice(&self.index.to_be_bytes());
        bytes.extend_from_slice(&self.begin.to_be_bytes());
        bytes.extend_from_slice(&self.length.to_be_bytes());
        bytes
    }
}

#[derive(Debug, Clone)]
pub struct ExtensionPayload {
    pub message_id: u8,
    pub payload: ExtensionPayloadData,
}

impl ExtensionPayload {
    pub fn new(message_id: u8, payload: ExtensionPayloadData) -> Self {
        Self {
            message_id,
            payload,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];
        bytes.extend_from_slice(&self.message_id.to_be_bytes());
        bytes.extend_from_slice(&self.payload.to_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let message_id = bytes[0];
        let payload = ExtensionPayloadData::from_bytes(&bytes[1..]);
        Self {
            message_id,
            payload,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExtensionPayloadData {
    pub m: ExtensionPayloadDataM,
}

impl ExtensionPayloadData {
    pub fn new(m: ExtensionPayloadDataM) -> Self {
        Self { m }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let data = serde_bencode::to_bytes(&self.m).unwrap();
        data
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let data: ExtensionPayloadData = serde_bencode::from_bytes(bytes).unwrap();
        data
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExtensionPayloadDataM {
    pub ut_metadata: u32,
}

impl ExtensionPayloadDataM {
    pub fn new(ut_metadata: u32) -> Self {
        Self { ut_metadata }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];
        bytes.extend_from_slice(&self.ut_metadata.to_be_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let data: ExtensionPayloadDataM = serde_bencode::from_bytes(bytes).unwrap();
        data
    }
}
