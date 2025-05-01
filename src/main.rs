use clap::Parser;
use gitdb::cli::commands::{self, CommandsWrapper, Commands};
use gitdb::core::database::CommitStorage;
use gitdb::core::branch::BranchManager;
use gitdb::error::GitDBError;
use std::fs;
use std::path::Path;

fn ensure_data_dir() -> Result<(), GitDBError> {
    if !Path::new("./data").exists() {
        fs::create_dir("./data").map_err(|e| GitDBError::InvalidInput(format!("Failed to create data dir: {}", e)))?;
    }
    Ok(())
}

fn run() -> Result<(), GitDBError> {
    ensure_data_dir()?;
    let args = CommandsWrapper::parse().command;
    
    // Open storage
    let storage = CommitStorage::open("./data")?;
    
    // Create branch manager with shared DB
    let branch_mgr = BranchManager::new(storage.db.clone());

    match args {
        Commands::Commit { message } => commands::handle_commit(&storage, &message),
        Commands::Branch { name, delete } => commands::handle_branch(&branch_mgr, &name, delete),
        Commands::Query { sql } => commands::handle_query(&sql, &storage.db),
        Commands::Sql { command } => commands::handle_sql(&storage, &command),
        Commands::ImportCsv { file, table } => commands::handle_import_csv(&storage, &file, &table),
        Commands::ShowTable { table_name, commit_hash } => {
            commands::handle_show_table(&*storage.db, &table_name, commit_hash.as_deref())
        }
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}