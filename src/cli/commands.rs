use clap::{Parser, Subcommand};
use crate::core::database::CommitStorage;
use crate::core::branch::BranchManager;
use crate::core::merge::merge_states;
use crate::core::query::QueryProcessor;
use crate::error::{BranchDBError, Result};
use rocksdb::DB;
use hex;
use csv;
use crate::core::models::Change;
use crate::core::crdt::{CrdtEngine, CrdtValue};
use std::path::Path;
use std::fs;
use std::collections::HashSet;

#[derive(Parser)]
pub struct CommandsWrapper {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Init {
        #[arg(help = "Path to initialize repository")]
        path: String,
    },

    Commit {
        #[arg(help = "Message to attach to the commit")]
        message: String,
    },
    Branch {
        #[arg(help = "Name of the branch to create or delete")]
        name: String,

        #[arg(short, long, help = "Delete the specified branch")]
        delete: bool,
    },
    Query {
        #[arg(help = "SQL query: SELECT * FROM <table> WITH <commit_hash>")]
        sql: String,
    },
    Sql {
        #[arg(help = "SQL command to execute (CREATE TABLE/INSERT INTO)")]
        command: String,
    },
    ImportCsv {
        #[arg(help = "Path to CSV file")]
        file: String,
        
        #[arg(help = "Target table name")]
        table: String,
    },
    ShowTable {
        #[arg(help = "Table name to display")]
        table_name: String,
        
        #[arg(long, help = "Commit hash to view at")]
        commit_hash: Option<String>,
    },
    Revert {
        #[arg(help = "Commit hash to revert to")]
        commit_hash: String,
    },
    
    Diff {
        #[arg(help = "First commit hash")]
        from: String,
        
        #[arg(help = "Second commit hash")]
        to: String,
    },
    
    History {
        #[arg(help = "Show commit history")]
        #[arg(short, long, help = "Limit number of commits")]
        limit: Option<usize>,
    },

    Checkout {
        #[arg(help = "Commit hash or branch name")]
        target: String,
    },
    // Show commit history
    Log {
        #[arg(short, long, help = "Show full details")]
        verbose: bool,
    },
    // Show list of branches
    /* 
    The command can now be used like:

    cargo run -- branch-list

    or with verbose output:

    cargo run -- branch-list --verbose
    */
    BranchList {
        #[arg(short, long, help = "Show additional branch information")]
        verbose: bool,
    },
    // Merge branches
    Merge {
        #[arg(help = "Branch name to merge")]
        branch: String,
    },
}

pub fn handle_commit(storage: &CommitStorage, message: &str) -> Result<()> {
    if message.trim().is_empty() {
        return Err(BranchDBError::InvalidInput("Commit message cannot be empty.".into()));
    }

    let changes = Vec::new();
    let hash = storage.create_commit(message, changes)?;
    println!("Created commit with hash: {}", hex::encode(hash));
    Ok(())
}

pub fn handle_branch(branch_mgr: &BranchManager, name: &str, delete: bool) -> Result<()> {
    if delete {
        branch_mgr.delete_branch(name)?;
        println!("Deleted branch '{}'.", name);
    } else {
        branch_mgr.create_branch(name)?;
        println!("Created branch '{}'.", name);
    }
    Ok(())
}

pub fn handle_query(sql: &str, db: &DB) -> Result<()> {
    let processor = QueryProcessor::new(db);
    processor.execute(sql)
}

pub fn handle_sql(storage: &CommitStorage, command: &str) -> Result<()> {
    let cmd_upper = command.to_uppercase();
    
    if cmd_upper.starts_with("CREATE TABLE") {
        let table_name = command.split_whitespace()
            .nth(2)
            .ok_or_else(|| BranchDBError::InvalidInput("Missing table name".into()))?;
        
        let changes = vec![Change::Insert {
            table: table_name.to_string(),
            id: "!schema".to_string(),
            value: bincode::serialize(&CrdtValue::Register(b"{}".to_vec()))?,
        }];
        
        storage.create_commit(&format!("SQL: {}", command), changes)?;
        Ok(())
    } 
    else if cmd_upper.starts_with("INSERT INTO") {
        let table = command.split_whitespace()
            .nth(2)
            .ok_or_else(|| BranchDBError::InvalidInput("Missing table name".into()))?;
        
        let values_start = command.find("VALUES")
            .ok_or_else(|| BranchDBError::InvalidInput("Missing VALUES clause".into()))? + 6;
        let values_part = &command[values_start..].trim();
        
        let values = parse_sql_values(values_part)?;
        if values.is_empty() {
            return Err(BranchDBError::InvalidInput("No values provided".into()));
        }
        
        let json_value = serde_json::to_string(&values)?;  
        
        let changes = vec![Change::Insert {
            table: table.to_string(),
            id: values[0].to_string(),
            value: bincode::serialize(&CrdtValue::Register(json_value.as_bytes().to_vec()))?,
        }];
        
        storage.create_commit(&format!("SQL: {}", command), changes)?;
        Ok(())
    }
    // Modified for proper SQL UPDATE command handling
    else if cmd_upper.starts_with("UPDATE") {
        let table = command.split_whitespace()
            .nth(1)
            .ok_or_else(|| BranchDBError::InvalidInput("Missing table name".into()))?;

        let set_idx = command.find("SET")
            .ok_or_else(|| BranchDBError::InvalidInput("Missing SET clause".into()))?;
        let where_idx = command.find("WHERE")
            .ok_or_else(|| BranchDBError::InvalidInput("Missing WHERE clause".into()))?;

        let set_clause = &command[set_idx+3..where_idx].trim();
        let where_clause = &command[where_idx+5..].trim();

        let id = where_clause.split("=")
            .nth(1)
            .ok_or_else(|| BranchDBError::InvalidInput("Invalid WHERE clause".into()))?
            .trim()
            .trim_matches('\'');

        // Create binary key without UTF-8 conversion
        let key = id.as_bytes().to_vec();

        // Get current value
        let current_value = match storage.db.get(&key)? {
            Some(existing) => {
                let mut current: serde_json::Value = match bincode::deserialize::<CrdtValue>(&existing)? {
                    CrdtValue::Register(data) => serde_json::from_slice(&data)?,
                    _ => return Err(BranchDBError::TypeMismatch("Expected Register type".into()))
                };
                
                // Apply updates
                for pair in set_clause.split(',') {
                    let mut parts = pair.split('=');
                    let field = parts.next()
                        .ok_or(BranchDBError::InvalidInput("Invalid SET clause".into()))?
                        .trim();
                    let value = parts.next()
                        .ok_or(BranchDBError::InvalidInput("Invalid SET clause".into()))?
                        .trim()
                        .trim_matches('\'');
                    current[field] = value.into();
                }
                current
            }
            None => {
                // Enhanced error reporting with hex-encoded keys
                let mut keys = Vec::new();
                let prefix = vec![];

                let iter = storage.db.prefix_iterator(&prefix);
                for item in iter {
                    let (raw_key, _) = item?;
                    let key_str = String::from_utf8_lossy(&raw_key);
                    if key_str == "!schema" {
                        continue; // Skip schema entry
                    }
                    keys.push(key_str.into_owned());
                }
                
                return Err(BranchDBError::InvalidInput(
                    format!("Row '{}' not found in table '{}'. Existing keys (hex): {:?}", 
                        id, table, keys)
                ));
            }
        };

        // Create and commit changes
        let changes = vec![Change::Update {
            table: table.to_string(),
            id: id.to_string(),
            value: bincode::serialize(&CrdtValue::Register(
                serde_json::to_vec(&current_value)?
            ))?,
        }];
        
        storage.create_commit(&format!("SQL: {}", command), changes)?;
        Ok(())
    }
    else {
        Err(BranchDBError::InvalidInput("Unsupported SQL command".into()))
    }
}

fn parse_sql_values(values_part: &str) -> Result<Vec<String>> {
    let mut values = Vec::new();
    let mut in_quotes = false;
    let mut current = String::new();
    let mut chars = values_part.chars().peekable();
    
    if values_part.starts_with('(') {
        chars.next();
    }
    
    while let Some(c) = chars.next() {
        match c {
            '\'' => {
                in_quotes = !in_quotes;
                if !in_quotes {
                    values.push(current.trim().to_string());
                    current.clear();
                }
            },
            ',' if !in_quotes => {
                // Skip commas between values
                while let Some(&next) = chars.peek() {
                    if next.is_whitespace() || next == ',' {
                        chars.next();
                    } else {
                        break;
                    }
                }
            },
            ')' if !in_quotes => break,
            _ => current.push(c),
        }
    }
    
    Ok(values)
}

pub fn handle_import_csv(storage: &CommitStorage, file: &str, table: &str) -> Result<()> {
    let mut rdr = csv::Reader::from_path(file)?;
    let headers = rdr.headers()?.clone();
    let mut changes = Vec::new();

    for result in rdr.records() {
        let record = result?;
        let id = record.get(0)
            .ok_or_else(|| BranchDBError::InvalidInput("CSV missing ID column".into()))?;
        
        let mut row = Vec::new();
        for (i, field) in record.iter().enumerate() {
            row.push(format!("\"{}\":\"{}\"", headers.get(i).unwrap_or(&i.to_string()), field));
        }
        
        changes.push(Change::Insert {
            table: table.to_string(),
            id: id.to_string(),
            value: bincode::serialize(&CrdtValue::Register(
                format!("{{{}}}", row.join(",")).as_bytes().to_vec()
            ))?,
        });
    }

    storage.create_commit(&format!("Import {} into {}", file, table), changes)?;
    Ok(())
}

pub fn handle_show_table(db: &DB, table_name: &str, commit_hash: Option<&str>) -> Result<()> {
    let processor = QueryProcessor::new(db);
    let hash = match commit_hash {
        Some(h) => hex::decode(h)?,
        None => processor.get_head_hash()?,
    };

    println!("Table '{}' at commit {}:", table_name, hex::encode(&hash));
    
    match processor.get_table_at_commit(table_name, &hash) {
        Ok(rows) => {
            // First print schema if it exists
            if let Some(CrdtValue::Register(schema_data)) = rows.get("!schema") {
                println!("Schema: {}", String::from_utf8_lossy(schema_data));
            }

            // Then print other rows
            for (id, value) in rows {
                if id == "!schema" {
                    continue;
                }
                match value {
                    CrdtValue::Register(data) => {
                        println!("{}: {}", id, String::from_utf8_lossy(&data));
                    }
                    CrdtValue::Counter(count) => {
                        println!("{}: {}", id, count);
                    }
                }
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("Showing partial data due to: {}", e);
            eprintln!("Falling back to direct table scan...");
            
            // Direct table scan fallback
            let iter = db.prefix_iterator(table_name.as_bytes());
            for item in iter {
                let (key, value) = item?;
                println!("{}: {}", 
                    String::from_utf8_lossy(&key),
                    String::from_utf8_lossy(&value));
            }
            Ok(())
        }
    }
}

pub fn handle_revert(storage: &CommitStorage, commit_hash: &str) -> Result<()> {
    // Validate commit hash format
    if commit_hash.len() != 64 {
        return Err(BranchDBError::InvalidInput(
            "Commit hash must be 64 characters long".into()
        ));
    }
    
    let hash_bytes = hex::decode(commit_hash)?;
    let hash_array: [u8; 32] = hash_bytes.try_into()
        .map_err(|_| BranchDBError::InvalidInput("Invalid commit hash".into()))?;
    
    // Verify the commit exists and show info
    let target_commit = storage.get_commit_by_hash(&hash_array)?;
    println!("Reverting to commit: {}", commit_hash);
    println!("Original commit message: {}", target_commit.message);
    println!("Date: {}", target_commit.timestamp);
    
    // Get current state before revert
    println!("\nCurrent state:");
    let before_state: Vec<_> = storage.db.iterator(rocksdb::IteratorMode::Start)
        .filter_map(|item| item.ok())
        .collect();
    
    // Filter and display only relevant table data (skip internal metadata)
    for (key, value) in &before_state {
        let key_str = String::from_utf8_lossy(key);
        if !key_str.starts_with("_internal") {  // Skip internal metadata
            println!("{}: {}", key_str, String::from_utf8_lossy(value));
        }
    }
    
    // Perform the revert
    storage.revert_to_commit(&hash_array)?;
    
    // Verify and show new state
    let current_head = storage.get_head()?
        .ok_or(BranchDBError::InvalidInput("No HEAD commit".into()))?;
    let current_commit = storage.get_commit_by_hash(&current_head)?;
    
    println!("\nSuccessfully reverted to commit {}", commit_hash);
    println!("New HEAD: {}", hex::encode(current_head));
    println!("New commit message: {}", current_commit.message);
    
    // Get state after revert
    println!("\nState after revert:");
    let after_state: Vec<_> = storage.db.iterator(rocksdb::IteratorMode::Start)
        .filter_map(|item| item.ok())
        .collect();
    
    // Filter and display only relevant table data
    for (key, value) in &after_state {
        let key_str = String::from_utf8_lossy(key);
        if !key_str.starts_with("_internal") {
            println!("{}: {}", key_str, String::from_utf8_lossy(value));
        }
    }
    
    // Compare states (only for user-visible data)
    println!("\nChanges:");
    let before_keys: HashSet<_> = before_state.iter()
        .filter(|(k, _)| !String::from_utf8_lossy(k).starts_with("_internal"))
        .map(|(k, _)| k)
        .collect();
        
    let after_keys: HashSet<_> = after_state.iter()
        .filter(|(k, _)| !String::from_utf8_lossy(k).starts_with("_internal"))
        .map(|(k, _)| k)
        .collect();
    
    // Added entries
    for key in after_keys.difference(&before_keys) {
        println!("+ {}", String::from_utf8_lossy(key));
    }
    
    // Removed entries
    for key in before_keys.difference(&after_keys) {
        println!("- {}", String::from_utf8_lossy(key));
    }
    
    // Changed entries
    for (key, after_value) in &after_state {
        let key_str = String::from_utf8_lossy(key);
        if !key_str.starts_with("_internal") {
            if let Some((_, before_value)) = before_state.iter().find(|(k, _)| k == key) {
                if before_value != after_value {
                    println!("≠ {} (changed)", key_str);
                }
            }
        }
    }
    
    Ok(())
}

pub fn handle_diff(storage: &CommitStorage, from: &str, to: &str) -> Result<()> {
    let from_bytes = hex::decode(from)?;
    let from_array: [u8; 32] = from_bytes.try_into()
        .map_err(|_| BranchDBError::InvalidInput("Invalid commit hash length".into()))?;
    
    let to_bytes = hex::decode(to)?;
    let to_array: [u8; 32] = to_bytes.try_into()
        .map_err(|_| BranchDBError::InvalidInput("Invalid commit hash length".into()))?;
    
    let diffs = storage.get_commit_diffs(&from_array, &to_array)?;
    
    println!("Changes from {} to {}:", from, to);
    for diff in diffs {
        println!("- {:?}", diff);
    }
    
    Ok(())
}

pub fn handle_history(storage: &CommitStorage, limit: Option<usize>) -> Result<()> {
    let history = storage.get_commit_history()?;
    
    let display_count = limit.unwrap_or(history.len());
    for commit in history.iter().take(display_count) {
        let hash = blake3::hash(&bincode::serialize(commit)?);
        println!("{}: {}", hex::encode(&hash.as_bytes()[..8]), commit.message);
        println!("  Date: {}", commit.timestamp);
        println!("  Changes: {}", commit.changes.len());
        println!();
    }
    
    Ok(())
}

pub fn handle_init(path: &str) -> Result<()> {
    if Path::new(path).exists() {
        return Err(BranchDBError::InvalidInput("Path already exists".into()));
    }
    
    fs::create_dir_all(path)?;
    let _storage = CommitStorage::open(path)?;
    println!("Initialized empty GitDB repository in {}", path);
    Ok(())
}

pub fn handle_checkout(storage: &CommitStorage, target: &str) -> Result<()> {
    // Try as branch first
    let branch_key = format!("branch:{}", target);
    if let Some(branch_head) = storage.db.get(branch_key.as_bytes())? {
        storage.db.put(b"HEAD", &branch_head)?;
        println!("Switched to branch '{}'", target);
        return Ok(());
    }

    // Try as commit hash
    if target.len() == 64 { // Basic hex hash check
        if let Ok(hash_bytes) = hex::decode(target) {
            if storage.db.get(&hash_bytes)?.is_some() {
                storage.db.put(b"HEAD", &hash_bytes)?;
                println!("Switched to commit {}", target);
                return Ok(());
            }
        }
    }

    Err(BranchDBError::InvalidInput(
        format!("No branch or commit found with reference '{}'", target)
    ))
}

pub fn handle_log(storage: &CommitStorage, verbose: bool) -> Result<()> {
    let mut current_hash = storage.get_head()?;
    
    while let Some(hash) = current_hash {
        let commit = storage.get_commit_by_hash(&hash)?;
        
        if verbose {
            println!("commit {}", hex::encode(&hash)); // Show full hash
            println!("Author: <user>");
            println!("Date:   {}", commit.timestamp);
            println!("\n    {}\n", commit.message);
        } else {
            println!("{} {}", hex::encode(&hash), commit.message); // Show full hash instead of short_hash
        }
        
        current_hash = commit.parents.get(0).cloned();
    }
    
    Ok(())
}

pub fn handle_branch_list(branch_mgr: &BranchManager, verbose: bool) -> Result<()> {
    let branches = branch_mgr.list_branches()?;
    let current = branch_mgr.get_current_branch()?;
    
    println!("Branches:");
    for branch in branches {
        if current.as_ref() == Some(&branch) {
            print!("* ");
        } else {
            print!("  ");
        }
        
        print!("{}", branch);
        
        if verbose {
            if let Some(commit_hash) = branch_mgr.get_branch_head(&branch)? {
                println!(" ({})", hex::encode(commit_hash));
            } else {
                println!(" (no commit)");
            }
        } else {
            println!();
        }
    }
    Ok(())
}

pub fn handle_merge(storage: &CommitStorage, branch_name: &str) -> Result<()> {
    let branch_key = format!("branch:{}", branch_name);
    let branch_head = storage.db.get(branch_key.as_bytes())?
        .ok_or_else(|| BranchDBError::InvalidInput(format!("Branch {} not found", branch_name)))?;
    
    let current_head = storage.db.get(b"HEAD")?
        .ok_or_else(|| BranchDBError::InvalidInput("HEAD not found".into()))?;
    
    if branch_head == current_head {
        return Err(BranchDBError::InvalidInput("Already up to date".into()));
    }
    
    let mut current_engine = CrdtEngine::new();
    let mut branch_engine = CrdtEngine::new();
    
    // Helper function to load state from a commit hash
    fn load_state(storage: &CommitStorage, mut hash: Vec<u8>, engine: &mut CrdtEngine) -> Result<()> {
        while !hash.is_empty() {
            // Convert Vec<u8> to [u8; 32]
            let hash_array: [u8; 32] = hash.as_slice().try_into()
                .map_err(|_| BranchDBError::InvalidInput("Invalid commit hash length".into()))?;
            
            let commit = storage.get_commit_by_hash(&hash_array)?;
            for change in &commit.changes {
                engine.apply_change(change)?;
            }
            hash = commit.parents.get(0).map(|p| p.to_vec()).unwrap_or_default();
        }
        Ok(())
    }
    
    // Load current branch state
    load_state(storage, current_head.to_vec(), &mut current_engine)?;
    
    // Load other branch state
    load_state(storage, branch_head.to_vec(), &mut branch_engine)?;
    
    // Merge the states
    let changes = merge_states(&mut current_engine, &branch_engine)?;
    
    if changes.is_empty() {
        println!("Already up to date");
        return Ok(());
    }
    
    // Create merge commit
    let hash = storage.create_commit(
        &format!("Merge branch '{}'", branch_name),
        changes
    )?;
    
    println!("Created merge commit: {}", hex::encode(hash));
    Ok(())
}