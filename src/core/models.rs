use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StructuredValue {
    Map(HashMap<String, serde_json::Value>),
    Array(Vec<serde_json::Value>),
    Primitive(serde_json::Value),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrdtValue {
    Counter(u64),
    Register(StructuredValue),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    pub parents: Vec<[u8; 32]>,
    pub message: String,
    pub timestamp: u64,
    pub changes: Vec<Change>,
    pub tree: HashMap<String, [u8; 32]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Change {
    Insert { 
        table: String, 
        id: String, 
        value: Vec<u8> 
    },
    Update { 
        table: String, 
        id: String, 
        value: Vec<u8> 
    },
    Delete { 
        table: String, 
        id: String 
    },
}

impl Change {
    pub fn table(&self) -> &str {
        match self {
            Change::Insert { table, .. } => table,
            Change::Update { table, .. } => table,
            Change::Delete { table, .. } => table,
        }
    }
}