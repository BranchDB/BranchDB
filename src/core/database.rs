use sled::Db;
use blake3;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::{
    error::Result, 
    core::models::{Commit, Change}, 
    error::GitDBError
};

pub struct CommitStorage {
    db: Db,
}

impl CommitStorage {
    pub fn open(path: &str) -> Result<Self> {
        Ok(Self {
            db: sled::open(path).map_err(|e| GitDBError::Storage(e))?
        })
    }

    pub fn create_commit(&self, message: &str, changes: Vec<Change>) -> Result<[u8; 32]> {  
        let parent = self.get_head()?.ok_or(GitDBError::OrphanCommit)?;
        
        let commit = Commit {
            parents: vec![parent],
            message: message.to_string(),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            changes,
        };
        
        let serialized = bincode::serialize(&commit)?;
        let hash = blake3::hash(&serialized);
        let hash_bytes: [u8; 32] = *hash.as_bytes();
        
        self.db.transaction(|tx| {
            tx.insert(&hash_bytes, serialized.as_slice())?;  
            tx.insert(b"HEAD", &hash_bytes)?;
            Ok(())
        })?;
        
        Ok(hash_bytes)
    }

    fn get_head(&self) -> Result<Option<[u8; 32]>> {
        Ok(self.db.get(b"HEAD")?
            .map(|ivec| {
                let mut bytes = [0; 32];
                bytes.copy_from_slice(&ivec);
                bytes
            }))
    }
}