use clap::Subcommand;
use crate::core::database::CommitStorage;
use crate::core::branch::BranchManager;
use crate::error::{Result, GitDBError};
use hex;

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new commit
    Commit {
        message: String,
    },
    
    /// Branch operations
    Branch {
        name: String,
        
        #[arg(short, long)]
        delete: bool,
    },
}

pub fn handle_commit(storage: &CommitStorage, message: &str) -> Result<()> {
    if message.is_empty() {
        return Err(GitDBError::InvalidInput("Commit message cannot be empty".into()));
    }
    
    let changes = vec![]; // Would collect changes interactively
    let hash = storage.create_commit(message, changes)?;
    println!("Created commit {}", hex::encode(&hash));
    Ok(())
}

pub fn handle_branch(branch_mgr: &BranchManager, name: &str, delete: bool) -> Result<()> {
    if delete {
        println!("Branch deletion coming in Checkpoint 2");
    } else {
        branch_mgr.create_branch(name)?;
        println!("Created branch '{}'", name);
    }
    Ok(())
}