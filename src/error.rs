use thiserror::Error;
use sled;
use bincode;
use gix::object::Error as GixError; 

#[derive(Debug, Error)]
pub enum GitDBError {
    #[error("Storage error: {0}")]
    Storage(#[from] sled::Error),
    
    #[error("Git operation failed: {0}")]
    Git(String),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),
    
    #[error("Branch '{0}' already exists")]
    BranchExists(String),
    
    #[error("Branch '{0}' not found")]
    BranchNotFound(String),
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    
    #[error("Commit has no parent")]
    OrphanCommit,
    
    #[error("Type mismatch: {0}")]
    TypeMismatch(String),
    
    #[error("Merge conflict: {0}")]
    MergeConflict(String),
}

impl From<GixError> for GitDBError {
    fn from(err: GixError) -> Self {
        GitDBError::Git(err.to_string())
    }
}

impl From<sled::transaction::TransactionError<sled::Error>> for GitDBError {
    fn from(err: sled::transaction::TransactionError<sled::Error>) -> Self {
        match err {
            sled::transaction::TransactionError::Storage(e) => GitDBError::Storage(e),
            sled::transaction::TransactionError::Abort(e) => GitDBError::Storage(e),
        }
    }
}

pub type Result<T> = std::result::Result<T, GitDBError>;