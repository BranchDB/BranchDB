use clap::Parser;
use gitdb::cli::commands::{self, CommandsWrapper, Commands};
use gitdb::core::database::CommitStorage;
use gitdb::core::branch::BranchManager;
use gitdb::error::GitDBError;
use std::fs;
use std::path::Path;

fn ensure_data_dir() -> std::io::Result<()> {
    if !Path::new("./data").exists() {
        fs::create_dir("./data")?;
    }
    Ok(())
}

fn run() -> Result<(), GitDBError> {
    ensure_data_dir().map_err(|e| GitDBError::InvalidInput(format!("Failed to create ./data dir: {}", e)))?;

    let args = CommandsWrapper::parse().command;

    // Open the storage first (this initializes the global DB)
    let storage = CommitStorage::open("./data")?;
    
    // Create BranchManager using the same DB instance
    let branch_mgr = BranchManager {
        db: storage.db,
    };

    match args {
        Commands::Commit { message } => {
            commands::handle_commit(&storage, &message)?;
        }
        Commands::Branch { name, delete } => {
            commands::handle_branch(&branch_mgr, &name, delete)?;
        }
        Commands::Query { sql } => {
            commands::handle_query(&sql, storage.db)?;
        }
    }

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}