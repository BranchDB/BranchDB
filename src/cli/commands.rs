use clap::{Parser, Subcommand};
use crate::core::database::CommitStorage;
use crate::core::branch::BranchManager;
use crate::core::query::QueryProcessor;
use crate::error::{GitDBError, Result};
use rocksdb::DB;
use hex;

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
}

pub fn handle_commit(storage: &CommitStorage, message: &str) -> Result<()> {
    if message.trim().is_empty() {
        return Err(GitDBError::InvalidInput("Commit message cannot be empty.".into()));
    }

    let changes = Vec::new(); // Placeholder
    let hash = storage.create_commit(message, changes)?;
    println!("Created commit with hash: {}", hex::encode(hash));
    Ok(())
}

pub fn handle_branch(branch_mgr: &BranchManager, name: &str, delete: bool) -> Result<()> {
    if delete {
        println!("Branch deletion not implemented yet.");
        return Ok(());
    }

    branch_mgr.create_branch(name)?;
    println!("Successfully created branch '{}'.", name);
    Ok(())
}

pub fn handle_query(sql: &str, db: &DB) -> Result<()> {
    let processor = QueryProcessor::new(db);
    processor.execute(sql)
}
