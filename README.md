# GitDB: A Git-Inspired Version-Controlled Database
**Group Name**:
- Version Controllers
**Members**:
- Avni Kanodia (`avnik2`)  
- Dibyadeep Saha (`saha18`)

---

## **Project Introduction**  
### What is GitDB?
GitDB is a **Rust-native database system** that brings Git's powerful version control capabilities to structured data management. It treats every database transaction as an immutable commit, enabling:

- 🕰️ **Full historical tracking** of all data changes
- 🌿 **Branching and merging** workflows for database content
- 🔄 **Automatic conflict resolution** using advanced CRDTs
- ⏱️ **Time-travel queries** to inspect past states

### Why GitDB Matters
Traditional databases force users to choose between:
1. Overwriting data (losing history)
2. Building complex audit systems (high maintenance)

GitDB provides **built-in version control** with familiar Git semantics, solving:
- 🛡️ **Data recovery** from accidental deletions
- 👥 **Team collaboration** through isolated branches
- 🔍 **Debugging** with precise change tracking
- 📜 **Compliance** with complete audit trails

---

## **Technical Overview**  

### **Core Components**  
| Component               | Description                                                                 | Rust Crates               |  
|-------------------------|-----------------------------------------------------------------------------|---------------------------|  
| **Commit Log**          | Append-only transaction store with cryptographic hashing                    | `sled`, `blake3`          |  
| **CRDT Engine**         | Resolves row-level conflicts automatically                                  | `crdt-rs`, `automerge-rs` |  
| **Branch Manager**      | Git-like refs for isolated workspaces                                       | `gitoxide`                |  
| **Query Processor**     | Time-travel SQL with historical state reconstruction                        | `sqlparser-rs`, `polars`  |  
| **Merge Engine**        | Three-way schema merging + CRDT data resolution                             | `similar` (diffs)         |  

### **Roadmap**  
#### **Checkpoint 1: Core Versioning & Branching**  
- [ ] Append-only commit storage (`sled` + `blake3`)  
- [ ] Row-level versioning with CRDTs (`crdt-rs`)  
- [ ] Basic branch creation/deletion (`gitoxide` integration)  
- [ ] CLI interface for commits/branches (`clap`)  

#### **Checkpoint 2: Merging & Query Interface**  
- [ ] Three-way schema merging (`similar` for diffing)  
- [ ] CRDT-based row conflict resolution  
- [ ] SQL time-travel parser (`sqlparser-rs`)  
- [ ] Historical state reconstruction (`polars` for query execution) 

---

## **Possible Challenges**  
1. **Merge Complexity**: Combining CRDTs with schema changes may require custom conflict resolution logic.  
2. **Storage Overhead**: Versioning every row could bloat storage (mitigation: delta encoding with `zstd`).  
3. **Query Performance**: Reconstructing historical states may be slow (solution: materialized snapshots).  
4. **Rust Learning Curve**: Working with `sled`’s atomic operations and `gitoxide`’s internals.  
