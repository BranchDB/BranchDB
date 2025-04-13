use clap::Parser;
use gitdb::cli::commands;
use gitdb::core::{CommitStorage, BranchManager};
use gitdb::error::GitDBError;

fn main() -> Result<(), GitDBError> {
    let args = commands::Commands::parse();
    
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