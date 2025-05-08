# GitDB - Version Controlled Database System

### Step-by-Step Setup
1. **Clone the repository**  
   `git clone https://github.com/[your-username]/gitdb.git`  
   `cd gitdb`  

2. **Build the project**  
   `cargo build --release`  

3. **Initialize the database**  
   `cargo run -- init`  

## Basic Usage
`cargo run -- [COMMAND] [ARGUMENTS]`  

## Available Commands

### Version Control Operations
| Command | Description | Example |
|---------|-------------|---------|
| `commit` | Record changes to the database | `cargo run -- commit -m "Added users table"` |
| `branch` | Create, list, or delete branches | `cargo run -- branch new-feature` |
| `checkout` | Switch between branches/commits | `cargo run -- checkout main` |
| `revert` | Revert to a previous state | `cargo run -- revert abc123` |
| `log` | Show commit history | `cargo run -- log --limit 5` |

### Data Operations
| Command | Description | Example |
|---------|-------------|---------|
| `sql` | Execute SQL commands | `cargo run -- sql "CREATE TABLE users (id TEXT PRIMARY KEY)"` |
| `import` | Import data from CSV | `cargo run -- import users.csv users` |
| `query` | Run queries (supports time-travel) | `cargo run -- query "SELECT * FROM users WITH abc123"` |
| `show` | Display table contents | `cargo run -- show users` |

### Advanced Features
| Command | Description | Example |
|---------|-------------|---------|
| `diff` | Compare two commits | `cargo run -- diff abc123 def456` |
| `merge` | Merge branches | `cargo run -- merge feature-branch` |
| `status` | Show current branch and changes | `cargo run -- status` |

## Example Workflow
1. **Initialize database**  
   `cargo run -- init`  

2. **Create table**  
   `cargo run -- sql "CREATE TABLE users (id TEXT, name TEXT)"`  

3. **Import data**  
   `cargo run -- import users.csv users`  

4. **Commit changes**  
   `cargo run -- commit -m "Initial user data import"`  

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
