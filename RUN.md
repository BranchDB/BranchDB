# GitDB - Version Controlled Database System

### Step-by-Step Setup
1. **Clone the repository**  
   `git clone https://github.com/AvniKanodia/rust-final-project`  
   `cd rust-final-project`  

2. **Build the project**  
   `cargo build --release`  

3. **Initialize the database**  
   `cargo run -- init [DATABASE NAME]`  

## Basic Usage
`cargo run -- [COMMAND] [ARGUMENTS]`  

## Available Commands

### Version Control Operations
| Command | Description | Example |
|---------|-------------|---------|
| `commit` | Record changes to the database | `cargo run -- commit -m "Added users table"` |
| `branch` | Create branches | `cargo run -- branch <name>` |
| `branch` | Delete branches | `cargo run --delete <name>` | 
| `checkout` | Switch between branches/commits | `cargo run -- checkout main` |
| `revert` | Revert to a previous state | `cargo run -- revert abc123` |
| `log` | Show commit history | `cargo run -- log --limit 5` |

### Data Operations
| Command | Description | Example |
|---------|-------------|---------|
| `sql` | Create SQL table | `cargo run -- sql "CREATE TABLE users (id TEXT, name TEXT)"` |
| `sql` | Insert data | `cargo run -- sql "INSERT INTO <table_name> VALUES ('value1', 'value2', ...)"` |
| `import` | Import data from CSV | `cargo run -- import users.csv users` |
| `show-table` | Display table contents | `cargo run -- show-table users` |

### Advanced Features
| Command | Description | Example |
|---------|-------------|---------|
| `diff` | Compare two commits | `cargo run -- diff abc123 def456` |
| `merge` | Merge branches | `cargo run -- merge feature-branch` |

## Example Workflow
1. **Initialize database**  
   `cargo run -- init`  

2. **Create table**  
   `cargo run -- sql "CREATE TABLE [DATATBASE NAME] (id TEXT, name TEXT)"`  

3. **Import data**  
   `cargo run -- import [CSV NAME].csv [CSV NAME]`  

4. **Commit changes**  
   `cargo run -- commit -m "[INSERT COMMIT MESSAGE]"`  

5. **Create branch**  
   `cargo run -- branch new-feature`  
   `cargo run -- checkout new-feature`  

6. **Merge changes**  
   `cargo run -- checkout main`  
   `cargo run -- merge new-feature`  

## Troubleshooting
- Update Rust: `rustup update`  
- Check dependencies: `cargo update`  
- Reset database: Delete `data/` and run `init`  

**Help command**  
`cargo run -- help`  
