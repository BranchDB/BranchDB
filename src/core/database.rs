use rocksdb::{DB, Options};
use blake3;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::core::models::{Commit, Change};
use crate::error::{BranchDBError, Result};
use std::sync::Arc;
use std::collections::HashMap;
use crate::core::crdt::{CrdtEngine, CrdtValue};

pub struct CommitStorage {
    pub db: Arc<DB>,
}

impl CommitStorage {
    pub fn open(path: &str) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let db = DB::open(&opts, path)?;
        Ok(Self {
            db: Arc::new(db)
        })
    }
    
    pub fn get_commit_by_hash(&self, hash: &[u8; 32]) -> Result<Commit> {
        let raw = self.db.get(hash)?
            .ok_or_else(|| BranchDBError::InvalidInput("Commit not found".into()))?;
        bincode::deserialize(&raw).map_err(Into::into)
    }

    pub fn get_head(&self) -> Result<Option<[u8; 32]>> {
        match self.db.get(b"HEAD")? {
            Some(raw) if raw.len() == 32 => {
                let mut bytes = [0u8; 32];
                bytes.copy_from_slice(&raw);
                Ok(Some(bytes))
            }
            Some(_) => Err(BranchDBError::InvalidInput("HEAD contains invalid data".into())),
            None => Ok(None),
        }
    }

    pub fn create_commit(&self, message: &str, changes: Vec<Change>) -> Result<[u8; 32]> {
        let parent = self.get_head()?;
        let mut tree = HashMap::new(); // Now defaults to HashMap<String, [u8; 32]>

        // Calculate content hashes for all tables
        for change in &changes {
            let table_hash = self.calculate_table_hash(change.table())?;
            tree.insert(change.table().to_string(), table_hash); // Convert &str to String
        }

        let commit = Commit {
            parents: parent.into_iter().collect(),
            message: message.to_string(),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            changes,
            tree, // Now correctly HashMap<String, [u8; 32]>
        };

        let serialized = bincode::serialize(&commit)?;
        let hash = blake3::hash(&serialized);
        let hash_bytes: [u8; 32] = *hash.as_bytes();

        // Verify we can deserialize immediately after serializing
        let test_deserialize: Commit = bincode::deserialize(&serialized)?;
        if test_deserialize.message != commit.message {
            return Err(BranchDBError::CorruptData("Serialization roundtrip failed".into()));
        }

        let checksum = blake3::hash(&serialized);
        let mut protected_value = serialized.clone();
        protected_value.extend_from_slice(checksum.as_bytes());

        // Store commit
        self.db.put(&hash_bytes, &protected_value)?;
        
        // Update HEAD
        self.update_head(&hash_bytes)?;
        
        Ok(hash_bytes)
    }

    pub fn revert_to_commit(&self, commit_hash: &[u8; 32]) -> Result<()> {
        // Verify commit exists
        let target_commit = self.get_commit_by_hash(commit_hash)?;
        
        // Create a new CRDT engine to build the target state
        let mut target_engine = CrdtEngine::new();
        
        // Apply all changes from the commit's history
        let mut current_hash = Some(*commit_hash);
        let mut commits_to_apply = Vec::new();
        
        // Walk the commit history
        while let Some(hash) = current_hash {
            let commit = self.get_commit_by_hash(&hash)?;
            commits_to_apply.push(commit.clone());
            current_hash = commit.parents.get(0).cloned();
        }
        
        // Apply changes in reverse order (oldest first)
        for commit in commits_to_apply.into_iter().rev() {
            for change in &commit.changes {
                target_engine.apply_change(change)?;
            }
        }
        
        // Clear ALL existing data for tables in the target commit
        let mut batch = rocksdb::WriteBatch::default();
        for table in target_commit.tree.keys() {
            let prefix = format!("{}:", table);
            let iter = self.db.prefix_iterator(prefix.as_bytes());
            for item in iter {
                let (key, _) = item?;
                batch.delete(key);
            }
        }
        
        // Write the new state
        for (table, rows) in target_engine.into_data() {
            for (id, value) in rows {
                let key = format!("{}:{}", table, id);
                let serialized = bincode::serialize(&value)?;
                batch.put(key.as_bytes(), serialized);
            }
        }
        
        // Create a revert commit
        let changes = target_commit.changes.iter()
            .map(|c| match c {
                Change::Insert { table, id, .. } => Change::Delete {
                    table: table.clone(),
                    id: id.clone(),
                },
                _ => c.clone(),
            })
            .collect();
        
        self.db.write(batch)?;
        self.create_commit(&format!("Revert to {}", hex::encode(commit_hash)), changes)?;
        
        Ok(())
    }

    fn calculate_table_hash(&self, table: &str) -> Result<[u8; 32]> {
        let mut hasher = blake3::Hasher::new();
        let mut rows = Vec::new();
        
        let iter = self.db.prefix_iterator(table.as_bytes());
        for result in iter {
            let (key, value) = result?;
            rows.push((key.to_vec(), value.to_vec()));
        }
        
        // Fix sorting with explicit types
        rows.sort_by(|a: &(Vec<u8>, Vec<u8>), b: &(Vec<u8>, Vec<u8>)| a.0.cmp(&b.0));
        
        for (key, value) in rows {
            hasher.update(&key);
            hasher.update(&value);
        }
        
        Ok(*hasher.finalize().as_bytes())
    }

    pub fn get_commit_diffs(&self, from: &[u8; 32], to: &[u8; 32]) -> Result<Vec<Change>> {
        let from_commit = self.get_commit_by_hash(from)?;
        let to_commit = self.get_commit_by_hash(to)?;
        
        let mut diffs = Vec::new();
        
        // Compare tables
        for (table, to_hash) in &to_commit.tree {
            if let Some(from_hash) = from_commit.tree.get(table) {
                if from_hash != to_hash {
                    // Get all changes for this table between commits
                    let table_diffs = self.get_table_diffs(table, from, to)?;
                    diffs.extend(table_diffs);
                }
            } else {
                // Table was added
                diffs.push(Change::Insert {
                    table: table.clone(),
                    id: "!schema".to_string(),
                    value: vec![],
                });
            }
        }
        Ok(diffs)
    }

    fn update_head(&self, hash: &[u8; 32]) -> Result<()> {
        self.db.put(b"HEAD", hash)?;
        Ok(())
    }

    pub fn get_commit_history(&self) -> Result<Vec<Commit>> {
        let mut history = Vec::new();
        let mut current_hash = self.get_head()?;

        while let Some(hash) = current_hash {
            let commit = self.get_commit_by_hash(&hash)?;
            history.push(commit.clone());
            current_hash = commit.parents.get(0).cloned();
        }

        Ok(history)
    }

    pub fn get_table_diffs(&self, table: &str, from: &[u8; 32], to: &[u8; 32]) -> Result<Vec<Change>> {
        let from_commit = self.get_commit_by_hash(from)?;
        let to_commit = self.get_commit_by_hash(to)?;
    
        // Get the state at each commit
        let mut from_engine = CrdtEngine::new();
        let mut to_engine = CrdtEngine::new();
    
        // Apply all changes up to 'from' commit
        let mut current_hash = from_commit.parents.get(0).cloned();
        while let Some(hash) = current_hash {
            let commit = self.get_commit_by_hash(&hash)?;
            for change in &commit.changes {
                if change.table() == table {
                    from_engine.apply_change(change)?;
                }
            }
            current_hash = commit.parents.get(0).cloned();
        }
    
        // Apply all changes up to 'to' commit
        let mut current_hash = to_commit.parents.get(0).cloned();
        while let Some(hash) = current_hash {
            let commit = self.get_commit_by_hash(&hash)?;
            for change in &commit.changes {
                if change.table() == table {
                    to_engine.apply_change(change)?;
                }
            }
            current_hash = commit.parents.get(0).cloned();
        }
    
        // Compare the states
        let mut diffs = Vec::new();
        let from_rows = from_engine.state.get(table).cloned().unwrap_or_default();
        let to_rows = to_engine.state.get(table).cloned().unwrap_or_default();
    
        // Find added/modified rows
        for (id, to_val) in &to_rows {
            match from_rows.get(id) {
                Some(from_val) if from_val != to_val => {
                    diffs.push(Change::Update {
                        table: table.to_string(),
                        id: id.clone(),
                        value: bincode::serialize(to_val)?,
                    });
                }
                None => {
                    diffs.push(Change::Insert {
                        table: table.to_string(),
                        id: id.clone(),
                        value: bincode::serialize(to_val)?,
                    });
                }
                _ => {}
            }
        }
    
        // Find deleted rows
        for (id, _) in from_rows {
            if !to_rows.contains_key(&id) {
                diffs.push(Change::Delete {
                    table: table.to_string(),
                    id,
                });
            }
        }
    
        Ok(diffs)
    }

    pub fn debug_commit(&self, hash: &str) -> Result<()> {
        let hash_bytes = hex::decode(hash)?;
        match self.db.get(&hash_bytes)? {
            Some(data) => {
                println!("Commit data ({} bytes):", data.len());
                println!("Hex: {}", hex::encode(&data));
                match bincode::deserialize::<Commit>(&data) {
                    Ok(commit) => println!("Valid commit: {:?}", commit),
                    Err(e) => println!("Deserialization failed: {}", e),
                }
            }
            None => println!("Commit not found"),
        }
        Ok(())
    }

    pub fn get_table_schema(&self, table: &str, commit_hash: Option<&[u8]>) -> Result<serde_json::Value> {
        // If no specific commit hash is provided, use the current state
        if commit_hash.is_none() {
            let key = format!("{}:!schema", table);
            if let Some(data) = self.db.get(key.as_bytes())? {
                return serde_json::from_slice(&data).map_err(Into::into);
            }
            return Ok(serde_json::json!({}));
        }

        // For historical schema lookups
        let hash = commit_hash.unwrap();
        let mut current_hash = hash.to_vec();
        
        while !current_hash.is_empty() {
            let hash_array: [u8; 32] = current_hash.as_slice().try_into()
                .map_err(|_| BranchDBError::InvalidInput("Invalid commit hash length".into()))?;
            
            let commit = self.get_commit_by_hash(&hash_array)?;
            
            // Check if this commit modified the schema
            for change in &commit.changes {
                if change.table() == table && matches!(change, Change::Update { id, .. } | Change::Insert { id, .. } if id == "!schema") {
                    if let Change::Insert { value, .. } | Change::Update { value, .. } = change {
                        let val: CrdtValue = bincode::deserialize(value)?;
                        if let CrdtValue::Register(data) = val {
                            return serde_json::from_slice(&data).map_err(Into::into);
                        }
                    }
                }
            }
            
            current_hash = commit.parents.get(0).map(|p| p.to_vec()).unwrap_or_default();
        }

        Ok(serde_json::json!({}))
    }
    
    pub fn update_table_schema(&self, table: &str, schema: &serde_json::Value) -> Result<()> {
        let key = format!("{}:!schema", table);
        self.db.put(key.as_bytes(), serde_json::to_vec(schema)?)?;
        Ok(())
    }
}