use sled::Db;
use blake3::{self, Hash};
use crate::{error::Result, core::models::{Commit, Change}, error::GitDBError};

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
            id: blake3::hash(&serialized_data),
            parent: parent,
            data: serialized_data,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        };
        
        let serialized = bincode::serialize(&commit)?;
        let hash = blake3::hash(&serialized);
        let hash_bytes: [u8; 32] = *hash.as_bytes();  // Convert to byte array
        
        // Store using the bytes directly
        self.db.transaction(|tx| {
            tx.insert(&hash_bytes, serialized)?;
            tx.insert(b"HEAD", &hash_bytes)?;
            Ok(())
        })?;
        
        Ok(hash_bytes)  // Return the byte array instead of Hash
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