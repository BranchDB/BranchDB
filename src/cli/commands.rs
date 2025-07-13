use clap::{Parser, Subcommand};
use crate::core::database::CommitStorage;
use crate::core::branch::BranchManager;
use crate::core::query::QueryProcessor;
use crate::error::{GitDBError, Result};
use rocksdb::DB;
use hex;
use csv;
use crate::core::models::Change;
use crate::core::crdt::CrdtValue;
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
        
        #[arg(help = "Commit hash to view at (defaults to HEAD)")]
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

    /// Show commit history
    Log {
        #[arg(short, long, help = "Show full details")]
        verbose: bool,
    },
}

pub fn handle_commit(storage: &CommitStorage, message: &str) -> Result<()> {
    if message.trim().is_empty() {
        return Err(GitDBError::InvalidInput("Commit message cannot be empty.".into()));
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
            .ok_or_else(|| GitDBError::InvalidInput("Missing table name".into()))?;
        
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
            .ok_or_else(|| GitDBError::InvalidInput("Missing table name".into()))?;
        
        let values_start = command.find("VALUES")
            .ok_or_else(|| GitDBError::InvalidInput("Missing VALUES clause".into()))? + 6;
        let values_part = &command[values_start..].trim();
        
        let values = parse_sql_values(values_part)?;
        if values.is_empty() {
            return Err(GitDBError::InvalidInput("No values provided".into()));
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
    else if cmd_upper.starts_with("UPDATE") {
        let table = command.split_whitespace()
            .nth(1)
            .ok_or_else(|| GitDBError::InvalidInput("Missing table name".into()))?;

        let set_idx = command.find("SET")
            .ok_or_else(|| GitDBError::InvalidInput("Missing SET clause".into()))?;
        let where_idx = command.find("WHERE")
            .ok_or_else(|| GitDBError::InvalidInput("Missing WHERE clause".into()))?;

        let set_clause = &command[set_idx+3..where_idx].trim();
        let where_clause = &command[where_idx+5..].trim();

        let id = where_clause.split("=")
            .nth(1)
            .ok_or_else(|| GitDBError::InvalidInput("Invalid WHERE clause".into()))?
            .trim()
            .trim_matches('\'');

        let updates: Vec<(&str, &str)> = set_clause.split(',')
            .filter_map(|pair| {
                let mut parts = pair.split('=');
                Some((
                    parts.next()?.trim(),
                    parts.next()?.trim().trim_matches('\'')
                ))
            })
            .collect();

        let json_value = serde_json::to_string(&updates)?;
        
        let changes = vec![Change::Update {
            table: table.to_string(),
            id: id.to_string(),
            value: bincode::serialize(&CrdtValue::Register(json_value.as_bytes().to_vec()))?,
        }];
        
        storage.create_commit(&format!("SQL: {}", command), changes)?;
        Ok(())
    }
    else {
        Err(GitDBError::InvalidInput("Unsupported SQL command".into()))
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
            .ok_or_else(|| GitDBError::InvalidInput("CSV missing ID column".into()))?;
        
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
        return Err(GitDBError::InvalidInput(
            "Commit hash must be 64 characters long".into()
        ));
    }
    
    let hash_bytes = hex::decode(commit_hash)?;
    let hash_array: [u8; 32] = hash_bytes.try_into()
        .map_err(|_| GitDBError::InvalidInput("Invalid commit hash".into()))?;
    
    // Verify the commit exists
    storage.get_commit_by_hash(&hash_array)?;
    
    // Get and print current state before revert
    println!("\nState before revert:");
    let before_state: Vec<_> = storage.db.iterator(rocksdb::IteratorMode::Start)
        .filter_map(|item| item.ok())
        .collect();
    
    for (key, value) in &before_state {
        println!("{}: {}", String::from_utf8_lossy(key), String::from_utf8_lossy(value));
    }
    
    storage.revert_to_commit(&hash_array)?;
    
    // Verify the revert worked
    let current_head = storage.get_head()?
        .ok_or(GitDBError::InvalidInput("No HEAD commit".into()))?;
    let current_commit = storage.get_commit_by_hash(&current_head)?;
    
    println!("\nSuccessfully reverted to commit {}", commit_hash);
    println!("Current HEAD: {}", hex::encode(current_head));
    println!("Commit message: {}", current_commit.message);
    
    // Print state after revert
    println!("\nState after revert:");
    let after_state: Vec<_> = storage.db.iterator(rocksdb::IteratorMode::Start)
        .filter_map(|item| item.ok())
        .collect();
    
    for (key, value) in &after_state {
        println!("{}: {}", String::from_utf8_lossy(key), String::from_utf8_lossy(value));
    }
    
    // Compare states
    println!("\nChanges:");
    let before_keys: HashSet<_> = before_state.iter().map(|(k, _)| k).collect();
    let after_keys: HashSet<_> = after_state.iter().map(|(k, _)| k).collect();
    
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
        if let Some((_, before_value)) = before_state.iter().find(|(k, _)| k == key) {
            if before_value != after_value {
                println!("≠ {} (changed)", String::from_utf8_lossy(key));
            }
        }
    }
    
    Ok(())
}

pub fn handle_diff(storage: &CommitStorage, from: &str, to: &str) -> Result<()> {
    let from_bytes = hex::decode(from)?;
    let from_array: [u8; 32] = from_bytes.try_into()
        .map_err(|_| GitDBError::InvalidInput("Invalid commit hash length".into()))?;
    
    let to_bytes = hex::decode(to)?;
    let to_array: [u8; 32] = to_bytes.try_into()
        .map_err(|_| GitDBError::InvalidInput("Invalid commit hash length".into()))?;
    
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
        return Err(GitDBError::InvalidInput("Path already exists".into()));
    }
    
    fs::create_dir_all(path)?;
    let _storage = CommitStorage::open(path)?;
    println!("Initialized empty GitDB repository in {}", path);
    Ok(())
}

pub fn handle_checkout(storage: &CommitStorage, target: &str) -> Result<()> {
    // Try as branch first
    if let Ok(Some(branch_data)) = storage.db.get(target.as_bytes()) {
        storage.db.put(b"HEAD", branch_data)?;
        println!("Switched to branch '{}'", target);
        return Ok(());
    }

    // Try as commit hash
    let hash_bytes = hex::decode(target)
        .map_err(|_| GitDBError::InvalidInput("Invalid commit hash or branch name".into()))?;
    
    if storage.db.get(&hash_bytes)?.is_none() {
        return Err(GitDBError::InvalidInput("Commit not found".into()));
    }

    storage.db.put(b"HEAD", &hash_bytes)?;
    println!("Switched to commit {}", target);
    Ok(())
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