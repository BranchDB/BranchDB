use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    pub parents: Vec<[u8; 32]>,
    pub message: String,
    pub timestamp: u64,
    pub changes: Vec<Change>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Change {
    Insert { table: String, id: String, value: Vec<u8> },
    Update { table: String, id: String, value: Vec<u8> },
    Delete { table: String, id: String },
}
