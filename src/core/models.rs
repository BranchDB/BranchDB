use serde::{Serialize, Deserialize};
use blake3::Hash;

#[derive(Debug, Serialize, Deserialize)]
pub enum Change {
    Insert { table: String, id: Vec<u8>, data: Vec<u8> },
    Update { table: String, id: Vec<u8>, data: Vec<u8> },
    Delete { table: String, id: Vec<u8> },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Commit {
    pub parents: Vec<[u8; 32]>,
    pub message: String,
    pub timestamp: u64,
    pub changes: Vec<Change>,
}