use crate::error::{GitDBError, Result};
use rocksdb::DB;

pub struct BranchManager {
    pub db: &'static DB,
}

impl BranchManager {
    pub fn create_branch(&self, name: &str) -> Result<()> {
        let head = self.db.get(b"HEAD").map_err(GitDBError::StorageError)?;
        let head = head.ok_or_else(|| GitDBError::InvalidInput("HEAD not found".into()))?;
        self.db.put(name.as_bytes(), head).map_err(GitDBError::StorageError)?;
        Ok(())
    }
}