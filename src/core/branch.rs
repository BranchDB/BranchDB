use gix::{Repository, refs::{FullNameRef, transaction::PreviousValue}};
use std::path::Path;
use crate::error::{Result, GitDBError};

pub struct BranchManager {
    repo: Repository,
}

impl BranchManager {
    pub fn open(path: &Path) -> Result<Self> {
        Ok(Self {
            repo: Repository::discover(path)
                .map_err(|e| GitDBError::Git(e.to_string()))?
        })
    }

    pub fn create_branch(&self, name: &str) -> Result<()> {
        if name.is_empty() {
            return Err(GitDBError::InvalidInput("Branch name cannot be empty".into()));
        }

        if self.branch_exists(name)? {
            return Err(GitDBError::BranchExists(name.to_string()));
        }

        let head = self.repo.head().map_err(|e| GitDBError::Git(e.to_string()))?;
        let commit = head.peel_to_commit_in_place()
            .map_err(|e| GitDBError::Git(e.to_string()))?
            .expect("HEAD should point to a commit");
        
        self.repo
            .reference(
                format!("refs/heads/{}", name).as_str(),
                commit.id(),
                PreviousValue::Any,
                "create branch",
            )
            .map_err(|e| GitDBError::Git(e.to_string()))?;
            
        Ok(())
    }

    fn branch_exists(&self, name: &str) -> Result<bool> {
        match self.repo.try_find_reference(&format!("refs/heads/{}", name)) {
            Ok(Some(_)) => Ok(true),
            Ok(None) => Ok(false),
            Err(e) => Err(GitDBError::Git(e.to_string())),
        }
    }
}