use rocksdb::{DB, Options};
use blake3;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::core::models::{Commit, Change};
use crate::error::{GitDBError, Result};
use once_cell::sync::OnceCell;

static GLOBAL_DB: OnceCell<DB> = OnceCell::new();

pub struct CommitStorage {
    pub db: &'static DB,
}

impl CommitStorage {
    pub fn open(path: &str) -> Result<Self> {
        let db = GLOBAL_DB.get_or_try_init(|| {
            let mut opts = Options::default();
            opts.create_if_missing(true);
            // Add these options to handle locking
            opts.set_use_fsync(false);
            opts.set_paranoid_checks(false);
            opts.set_skip_checking_sst_file_sizes_on_db_open(true);
            
            DB::open(&opts, path).map_err(GitDBError::StorageError)
        })?;
    
        Ok(Self { db })
    }

    pub fn create_commit(&self, message: &str, changes: Vec<Change>) -> Result<[u8; 32]> {
        let parent = self.get_head()?; 
    
        let commit = Commit {
            parents: match parent {
                Some(p) => vec![p],
                None => Vec::new(), // First commit has no parents
            },
            message: message.to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("System time went backwards")
                .as_secs(),
            changes,
        };
    
        let serialized = bincode::serialize(&commit)?;
        let hash = blake3::hash(&serialized);
        let hash_bytes: [u8; 32] = *hash.as_bytes();
    
        self.db.put(&hash_bytes, serialized).map_err(GitDBError::StorageError)?;
        self.db.put(b"HEAD", &hash_bytes).map_err(GitDBError::StorageError)?;
    
        Ok(hash_bytes)
    }
    
    fn get_head(&self) -> Result<Option<[u8; 32]>> {
        match self.db.get(b"HEAD").map_err(GitDBError::StorageError)? {
            Some(raw) if raw.len() == 32 => {
                let mut bytes = [0u8; 32];
                bytes.copy_from_slice(&raw);
                Ok(Some(bytes))
            }
            Some(_) => Err(GitDBError::InvalidInput("HEAD contains invalid data".into())),
            None => Ok(None), // No HEAD exists yet (first commit)
        }
    }
}
