#[derive(Debug)]
pub struct PeerMessage {
    pub length: [u8; 4],
    pub message_id: [u8; 1],
    pub payload: Vec<u8>,
}

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
