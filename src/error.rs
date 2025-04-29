use thiserror::Error;
use rocksdb;
use bincode;

#[derive(Debug, Error)]
pub enum GitDBError {
    #[error("Storage error: {0}")]
    StorageError(#[from] rocksdb::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Commit has no parent")]
    OrphanCommit,

    #[error("Type mismatch: {0}")]
    TypeMismatch(String),
}

pub type Result<T> = std::result::Result<T, GitDBError>;
