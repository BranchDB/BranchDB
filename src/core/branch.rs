use crate::error::{BranchDBError, Result};
use rocksdb::DB;
use std::sync::Arc;

// BranchManager handles creation and deletion of branches in the BranchDB database. Each branch points to a commit hash.
pub struct BranchManager {
    pub db: Arc<DB>,
}

impl BranchManager {
    pub fn new(db: Arc<DB>) -> Self {
        Self { db }
    }

    pub fn create_branch(&self, name: &str) -> Result<()> {
        if name.trim().is_empty() {
            return Err(BranchDBError::InvalidInput("Branch name cannot be empty".into()));
        }

        let branch_key = format!("branch:{}", name);
        if self.db.get(branch_key.as_bytes())?.is_some() {
            return Err(BranchDBError::InvalidInput(format!("Branch '{}' already exists", name)));
        }

        let head = self.db.get(b"HEAD")?.ok_or_else(|| {
            BranchDBError::InvalidInput(format!("Cannot create branch '{}': HEAD not found", name))
        })?;

        self.db.put(branch_key.as_bytes(), head)?;
        Ok(())
    }

    pub fn delete_branch(&self, name: &str) -> Result<()> {
        let branch_key = format!("branch:{}", name);
        if self.db.get(branch_key.as_bytes())?.is_none() {
            return Err(BranchDBError::InvalidInput(format!("Branch '{}' does not exist", name)));
        }

        self.db.delete(branch_key.as_bytes())?;
        println!("Deleted branch '{}" , name);
        Ok(())
    }

    pub fn list_branches(&self) -> Result<Vec<String>> {
        let mut branches = Vec::new();
        let iter = self.db.prefix_iterator("branch:");
        for item in iter {
            let (key, _) = item?;
            let branch_name = String::from_utf8_lossy(&key["branch:".len()..]).into_owned();
            branches.push(branch_name);
        }
        Ok(branches)
    }
    
    pub fn get_current_branch(&self) -> Result<Option<String>> {
        if let Some(head) = self.db.get(b"HEAD")? {
            let iter = self.db.prefix_iterator("branch:");
            for item in iter {
                let (key, value) = item?;
                if &value[..] == &head[..] {  // Compare slices of the underlying bytes
                    let branch_name = String::from_utf8_lossy(&key["branch:".len()..]).into_owned();
                    return Ok(Some(branch_name));
                }
            }
        }
        Ok(None)
    }

    pub fn get_branch_head(&self, branch_name: &str) -> Result<Option<Vec<u8>>> {
        let branch_key = format!("branch:{}", branch_name);
        Ok(self.db.get(branch_key.as_bytes())?.map(|v| v.to_vec()))
    }
}