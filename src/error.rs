use thiserror::Error;
use sled;
use bincode;
use gix::Error as GixError;
use std::fmt;

#[derive(Error, Debug)]
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

impl From<Box<bincode::ErrorKind>> for GitDBError {
    fn from(err: Box<bincode::ErrorKind>) -> Self {
        GitDBError::Serialization(*err)
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

impl fmt::Display for GitDBError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitDBError::Storage(e) => write!(f, "Storage error: {}", e),
            GitDBError::Git(e) => write!(f, "Git operation failed: {}", e),
            GitDBError::Serialization(e) => write!(f, "Serialization error: {}", e),
            GitDBError::BranchExists(name) => write!(f, "Branch '{}' already exists", name),
            GitDBError::BranchNotFound(name) => write!(f, "Branch '{}' not found", name),
            GitDBError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            GitDBError::OrphanCommit => write!(f, "Commit has no parent"),
            GitDBError::TypeMismatch(msg) => write!(f, "Type mismatch: {}", msg),
            GitDBError::MergeConflict(msg) => write!(f, "Merge conflict: {}", msg),
        }
    }
}

pub type Result<T> = std::result::Result<T, GitDBError>;