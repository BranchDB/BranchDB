use clap::Parser;
use gitdb::{cli::commands, core::{CommitStorage, BranchManager}};
use crate::error::GitDBError;

fn main() -> Result<(), GitDBError> {
    let args = commands::Commands::parse();
    
    // Initialize components
    let storage = CommitStorage::open("./data")?;
    let branch_mgr = BranchManager::open("./data".as_ref())?;

    match args {
        commands::Commands::Commit { message } => {
            commands::handle_commit(&storage, &message)?;
        }
        commands::Commands::Branch { name, delete } => {
            commands::handle_branch(&branch_mgr, &name, delete)?;
        }
    }
    
    Ok(())
}