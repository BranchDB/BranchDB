# BranchDB: A Git-Inspired Version-Controlled Database

---

## **Project Introduction**  
### What is BranchDB?
BranchDB is a **Rust-native database system** that brings Git's powerful version control capabilities to structured data management. It treats every database transaction as an immutable commit, enabling:

- **Full historical tracking** of all data changes
- **Branching and merging** workflows for database content
- **Automatic conflict resolution** using CRDTs
- **Time-travel queries** to inspect past states

### Why BranchDB Matters
Traditional databases force users to choose between:
1. Overwriting data (losing history)
2. Building complex audit systems (high maintenance)

BranchDB provides **built-in version control** with familiar Git semantics, solving:
- **Data recovery** from accidental deletions
- **Team collaboration** through isolated branches
- **Debugging** with precise change tracking

---

## **Technical Overview**  

### **Core Components**  
| Component               | Description                                                                 | Implementation           |  
|-------------------------|-----------------------------------------------------------------------------|--------------------------|  
| **Storage Engine**      | Key-value storage with versioned data                                       | `rocksdb`                |  
| **Commit System**       | Immutable commits with BLAKE3 hashing                                       | `blake3`, `bincode`      |  
| **CRDT Engine**         | Resolves row-level conflicts automatically                                  | Custom implementation    |  
| **Branch Manager**      | Branch references                                                           | `rocksdb`                |  
| **Query Interface**     | SQL-like queries                                                            | `sqlparser`              |  
| **Data Import**         | CSV input with version tracking                                             | `csv` crate              |  

### **Current Functionality**
#### **Version Control Features**
- Create, view, and revert commits
- Branch creation and switching
- Cryptographic commit hashes
- Time-travel data access

#### **Data Operations**
- Table creation via SQL
- Row-level inserts/updates/deletes
- CSV data import
- Versioned data queries
- Commit diffs and comparisons

#### **Conflict Resolution**
- Last-writer-wins register CRDTs
- Max-value counter CRDTs
- Automatic merge conflict resolution
- Schema change tracking

---

#### **Future Enhancements**
- Three-way merge capabilities
- Advanced query optimization
- Performance benchmarking
- Web-based visualization interface
