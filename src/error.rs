use std::fmt;
use std::time::SystemTimeError;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub enum BranchDBError {
    StorageError(String),          // Changed from rocksdb::Error
    InvalidInput(String),
    OrphanCommit,
    TypeMismatch(String),
    SerializationError(String),    // Changed from Box<bincode::ErrorKind>
    CsvError(String),             // Changed from csv::Error
    HexError(String),             // Changed from hex::FromHexError
    IoError(String),
    JsonError(String),            // Changed from serde_json::Error
    CorruptData(String),
}

pub type Result<T, E = BranchDBError> = std::result::Result<T, E>;

impl fmt::Display for BranchDBError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BranchDBError::StorageError(e) => write!(f, "Storage error: {}", e),
            BranchDBError::InvalidInput(s) => write!(f, "Invalid input: {}", s),
            BranchDBError::OrphanCommit => write!(f, "Commit has no parent"),
            BranchDBError::TypeMismatch(s) => write!(f, "Type mismatch: {}", s),
            BranchDBError::SerializationError(e) => write!(f, "Serialization error: {}", e),
            BranchDBError::CsvError(e) => write!(f, "CSV error: {}", e),
            BranchDBError::HexError(e) => write!(f, "Hex conversion error: {}", e),
            BranchDBError::IoError(s) => write!(f, "IO error: {}", s),
            BranchDBError::JsonError(e) => write!(f, "JSON error: {}", e),
            BranchDBError::CorruptData(s) => write!(f, "Data corruption detected: {}", s),
        }
    }
}

// Conversion implementations
impl From<rocksdb::Error> for BranchDBError {
    fn from(err: rocksdb::Error) -> Self {
        BranchDBError::StorageError(err.to_string())
    }
}

impl From<Box<bincode::ErrorKind>> for BranchDBError {
    fn from(err: Box<bincode::ErrorKind>) -> Self {
        BranchDBError::SerializationError(err.to_string())
    }
}

impl From<csv::Error> for BranchDBError {
    fn from(err: csv::Error) -> Self {
        BranchDBError::CsvError(err.to_string())
    }
}

impl From<hex::FromHexError> for BranchDBError {
    fn from(err: hex::FromHexError) -> Self {
        BranchDBError::HexError(err.to_string())
    }
}

impl From<std::io::Error> for BranchDBError {
    fn from(err: std::io::Error) -> Self {
        BranchDBError::IoError(err.to_string())
    }
}

impl From<serde_json::Error> for BranchDBError {
    fn from(err: serde_json::Error) -> Self {
        BranchDBError::JsonError(err.to_string())
    }
}

impl From<SystemTimeError> for BranchDBError {
    fn from(err: SystemTimeError) -> Self {
        BranchDBError::IoError(err.to_string())
    }
}