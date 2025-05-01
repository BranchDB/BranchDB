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
        let head = self.db.get(b"HEAD")?
            .ok_or_else(|| GitDBError::InvalidInput("HEAD not found".into()))?;
        self.db.put(name.as_bytes(), head)?;
        Ok(())
    }

    pub fn delete_branch(&self, name: &str) -> Result<()> {
        self.db.delete(name.as_bytes())?;
        Ok(())
    }
}