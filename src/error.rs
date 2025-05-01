use std::fmt;

#[derive(Debug)]
pub enum GitDBError {
    StorageError(rocksdb::Error),
    InvalidInput(String),
    OrphanCommit,
    TypeMismatch(String),
    SerializationError(Box<bincode::ErrorKind>),
    CsvError(csv::Error),
    HexError(hex::FromHexError),
    IoError(String),
    JsonError(serde_json::Error),  
}

pub type Result<T> = std::result::Result<T, GitDBError>;

impl fmt::Display for GitDBError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GitDBError::StorageError(e) => write!(f, "Storage error: {}", e),
            GitDBError::InvalidInput(s) => write!(f, "Invalid input: {}", s),
            GitDBError::OrphanCommit => write!(f, "Commit has no parent"),
            GitDBError::TypeMismatch(s) => write!(f, "Type mismatch: {}", s),
            GitDBError::SerializationError(e) => write!(f, "Serialization error: {}", e),
            GitDBError::CsvError(e) => write!(f, "CSV error: {}", e),
            GitDBError::HexError(e) => write!(f, "Hex conversion error: {}", e),
            GitDBError::IoError(s) => write!(f, "IO error: {}", s),
            GitDBError::JsonError(e) => write!(f, "JSON error: {}", e),  // Added this match arm
        }
    }
}

impl From<rocksdb::Error> for GitDBError {
    fn from(err: rocksdb::Error) -> GitDBError {
        GitDBError::StorageError(err)
    }
}

impl From<Box<bincode::ErrorKind>> for GitDBError {
    fn from(err: Box<bincode::ErrorKind>) -> GitDBError {
        GitDBError::SerializationError(err)
    }
}

impl From<csv::Error> for GitDBError {
    fn from(err: csv::Error) -> GitDBError {
        GitDBError::CsvError(err)
    }
}

impl From<hex::FromHexError> for GitDBError {
    fn from(err: hex::FromHexError) -> GitDBError {
        GitDBError::HexError(err)
    }
}

impl From<std::io::Error> for GitDBError {
    fn from(err: std::io::Error) -> GitDBError {
        GitDBError::IoError(err.to_string())
    }
}

impl From<serde_json::Error> for GitDBError {
    fn from(err: serde_json::Error) -> GitDBError {
        GitDBError::JsonError(err)  // Added this implementation
    }
}