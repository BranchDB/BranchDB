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

#[derive(Parser)]
pub struct CommandsWrapper {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
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
    let rows = processor.get_table_at_commit(table_name, &hash)?;

    for (id, value) in rows {
        if let CrdtValue::Register(data) = value {
            println!("{}: {}", id, String::from_utf8_lossy(&data));
        }
    }
    Ok(())
}