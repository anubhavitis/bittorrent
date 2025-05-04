use crate::handshake::HandshakeMessage;
use crate::peer::Peer;
use crate::pieces::Piece;
use crate::tcp::TcpClient;
use crate::torrent::Torrent;
use std::io::Write;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::{collections::HashMap, fs::File};
use tokio::sync::{mpsc, Semaphore};

struct FetchedData {
    piece_index: usize,
    data: Vec<u8>,
}

impl FetchedData {
    pub fn new(piece_index: usize, data: Vec<u8>) -> Self {
        Self { piece_index, data }
    }
}

pub struct Client {
    torrent: Torrent,
    peers: Vec<Peer>,
    pieces: Vec<Piece>,
    piece_peers: Vec<Vec<Peer>>,
    fetched_data: Vec<FetchedData>,

    streams: HashMap<SocketAddr, TcpClient>,
}

impl Client {
    pub fn new(torrent: Torrent) -> Self {
        Self {
            torrent,
            peers: Vec::new(),
            pieces: Vec::new(),
            piece_peers: Vec::new(),
            fetched_data: Vec::new(),
            streams: HashMap::new(),
        }
    }

    pub async fn init_download(&mut self) {
        self.init_peers().await;
        self.init_pieces();
        self.init_piece_peers();
    }

    pub async fn handshake(&mut self, peer: SocketAddr) -> String {
        let handshake_message = HandshakeMessage::new(self.torrent.get_info_hash());
        let mut client = TcpClient::new(peer).await;
        client.handshake(handshake_message).await
    }

    pub fn init_piece_peers(&mut self) {
        self.piece_peers = self
            .pieces
            .iter()
            .map(|piece| {
                self.peers
                    .iter()
                    .filter(|peer| peer.has_index(piece.index))
                    .cloned()
                    .collect()
            })
            .collect();
    }

    pub fn init_pieces(&mut self) {
        let piece_hashes = self.torrent.get_piece_hashes();
        let general_piece_length: u32 = self.torrent.info.piece_length;
        let total_length = self.torrent.info.length;

        for (i, hash) in piece_hashes.iter().enumerate() {
            let length = if i == piece_hashes.len() - 1 {
                total_length - i as u32 * general_piece_length
            } else {
                general_piece_length
            };

            self.pieces.push(Piece::new(i, hash.clone(), length));
        }
    }

    pub async fn init_peers(&mut self) {
        let peers = self.torrent.get_peers().await.expect("Failed to get peers");
        let handshake_message = HandshakeMessage::new(self.torrent.get_info_hash());

        let semaphore = Arc::new(Semaphore::new(10));
        let (test_sender, mut test_receiver) = mpsc::channel(10);

        for peer in peers {
            let sender = test_sender.clone();
            let semaphore = semaphore.clone();
            let handshake_message = handshake_message.clone();

            tokio::spawn(async move {
                let permit = semaphore.acquire().await.unwrap();
                let mut client = TcpClient::new(peer).await;
                let _ = client.handshake(handshake_message).await;
                let bitfield_buffer = client.read_message().await;
                assert_eq!(bitfield_buffer[0], 5);
                let bitfield = bitfield_buffer[1..].to_vec();

                let new_peer = Peer::new(peer, bitfield);
                sender.send((new_peer, client)).await.unwrap();

                permit.forget();
            });
        }

        drop(test_sender);
        while let Some((peer, client)) = test_receiver.recv().await {
            self.peers.push(peer.clone());
            self.streams.insert(peer.addr, client);
        }
    }

    pub async fn download(&mut self, save_path: PathBuf, only_piece_index: Option<usize>) {
        self.init_download().await;
        let (data_sender, mut data_receiver) = mpsc::channel(1 << 15);
        let semaphore = Arc::new(Semaphore::new(5));

        for (piece_index, piece_peer) in self.piece_peers.iter().enumerate() {
            if let Some(only_piece_index) = only_piece_index {
                if only_piece_index != piece_index {
                    continue;
                }
            }
            let handshake_message = HandshakeMessage::new(self.torrent.get_info_hash());
            let data_sender = data_sender.clone();
            let semaphore = semaphore.clone();
            let peer = piece_peer[0].clone();
            let piece_length = self.pieces[piece_index].length;
            let hash = self.pieces[piece_index].hash.clone();
            tokio::spawn(async move {
                let permit = semaphore.acquire().await.unwrap();
                // eprintln!(
                //     "Downloading piece {} of len {} from peer {}",
                //     piece_index, piece_length, peer.addr
                // );
                let mut client = TcpClient::new(peer.addr).await;

                let _ = client.handshake(handshake_message).await;
                let data = client.download_piece(piece_index, piece_length, hash).await;
                data_sender
                    .send(FetchedData::new(
                        piece_index,
                        data.expect("Failed to download piece"),
                    ))
                    .await
                    .unwrap();
                permit.forget();
            });
        }

        drop(data_sender);
        while let Some(data) = data_receiver.recv().await {
            // eprintln!("Received data for piece {}", data.piece_index);
            self.fetched_data.push(data);
        }

        self.fetched_data.sort_by_key(|data| data.piece_index);
        let mut file = File::create(save_path).unwrap();
        for data in self.fetched_data.iter() {
            file.write_all(&data.data).unwrap();
        }

        file.flush().unwrap();
    }
}
