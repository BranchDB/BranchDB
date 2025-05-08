use crate::error::{GitDBError, Result};
use rocksdb::DB;
use std::sync::Arc;

pub struct BranchManager {
    pub db: Arc<DB>,
}

impl BranchManager {
    pub fn new(db: Arc<DB>) -> Self {
        Self { db }
    }

    pub fn create_branch(&self, name: &str) -> Result<()> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(GitDBError::InvalidInput("Branch name cannot be empty".into()));
        }

        let branch_key = format!("branch:{}", trimmed);
        if self.db.get(branch_key.as_bytes())?.is_some() {
            return Err(GitDBError::InvalidInput(format!("Branch '{}' already exists", trimmed)));
        }

        let head = self.db.get(b"HEAD")?.ok_or_else(|| {
            GitDBError::InvalidInput(format!("Cannot create branch '{}'", trimmed))
        })?;

        self.db.put(branch_key.as_bytes(), head)?;
        println!("Created new branch '{}" , trimmed);
        Ok(())
    }

    pub fn delete_branch(&self, name: &str) -> Result<()> {
        let branch_key = format!("branch:{}", name);
        if self.db.get(branch_key.as_bytes())?.is_none() {
            return Err(GitDBError::InvalidInput(format!("Branch '{}' does not exist", name)));
        }

        self.db.delete(branch_key.as_bytes())?;
        println!("Deleted branch '{}" , name);
        Ok(())
    }
}