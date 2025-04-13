use clap::Subcommand;
use core::database::CommitStorage;
use core::branch::BranchManager;
use error::Result;
use hex;
use gix::bstr::ByteSlice;

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
    println!("Created commit {}", hex::encode(hash.as_bytes()));
    Ok(())
}

pub fn handle_branch(branch_mgr: &BranchManager, name: &str, delete: bool) -> Result<()> {
    if delete {
        // Delete branch implementation for Checkpoint 2
        println!("Branch deletion coming in Checkpoint 2");
    } else {
        branch_mgr.create_branch(name)?;
        println!("Created branch '{}'", name);
    }
    Ok(())
}