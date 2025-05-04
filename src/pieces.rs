#[derive(Debug, Clone)]
pub struct Piece {
    pub index: usize,
    pub hash: String,
    pub length: u32,
}

impl Piece {
    pub fn new(index: usize, hash: String, length: u32) -> Self {
        Self {
            index,
            hash,
            length,
        }
    }
}
