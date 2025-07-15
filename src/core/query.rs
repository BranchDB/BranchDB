use crate::core::models::Commit;
use crate::core::crdt::CrdtEngine;
use crate::error::{BranchDBError, Result};
use rocksdb::DB;
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;
use sqlparser::ast::{Statement, Query, SetExpr};
use std::collections::HashMap;
use crate::core::crdt::CrdtValue;

pub struct QueryProcessor<'a> {
    db: &'a DB
}

impl<'a> QueryProcessor<'a> {
    pub fn new(db: &'a DB) -> Self {
        QueryProcessor { db }
    }

    pub fn execute(&self, sql: &str) -> Result<()> {
        let dialect = GenericDialect;
        let ast = Parser::parse_sql(&dialect, sql)
            .map_err(|e| BranchDBError::InvalidInput(format!("SQL parse error: {}", e)))?;

        if ast.len() != 1 {
            return Err(BranchDBError::InvalidInput("Only one SQL statement is allowed".into()));
        }

        let Statement::Query(query) = &ast[0] else {
            return Err(BranchDBError::InvalidInput("Only SELECT queries are supported".into()));
        };

        let (table, commit_hash) = Self::extract_table_and_commit(query)?;
        let commit = self.get_commit_by_hash(&commit_hash)?;

        let mut engine = CrdtEngine::new();
        for change in &commit.changes {
            engine.apply_change(change)?;
        }

        if let Some(rows) = engine.into_data().remove(&table) {
            for (id, value) in rows {
                println!("{:?}: {:?}", id, value);
            }
        } else {
            println!("No rows found for table '{}'.", table);
        }

        Ok(())
    }

    fn extract_table_and_commit(query: &Query) -> Result<(String, String)> {
        let SetExpr::Select(select) = &*query.body else {
            return Err(BranchDBError::InvalidInput("Expected SELECT statement".into()));
        };

        let from = select.from.get(0)
            .ok_or_else(|| BranchDBError::InvalidInput("Missing FROM clause".into()))?;

        let table_name = from.relation.to_string();

        let Some(with) = &query.with else {
            return Err(BranchDBError::InvalidInput("Missing WITH clause".into()));
        };

        let cte = with.cte_tables.get(0)
            .ok_or_else(|| BranchDBError::InvalidInput("Missing CTE in WITH clause".into()))?;

        let commit_hash = cte.alias.name.to_string();
        Ok((table_name, commit_hash))
    }

    fn get_commit_by_hash(&self, hex_hash: &str) -> Result<Commit> {
        let hash_bytes = hex::decode(hex_hash)
            .map_err(|_| BranchDBError::InvalidInput("Invalid hex string for commit hash".into()))?;

        let raw = self.db.get(&hash_bytes).map_err(|e| BranchDBError::StorageError(e.to_string()))?
            .ok_or_else(|| BranchDBError::InvalidInput("Commit not found".into()))?;

        let commit: Commit = bincode::deserialize(&raw)?;
        Ok(commit)
    }

    pub fn get_table_at_commit(&self, table: &str, commit_hash: &[u8]) -> Result<HashMap<String, CrdtValue>> {
        // Simple validation
        if commit_hash.is_empty() {
            return Err(BranchDBError::InvalidInput("Empty commit hash".into()));
        }
    
        let mut engine = CrdtEngine::new();
        let mut current_hash = commit_hash.to_vec();
        
        while !current_hash.is_empty() {
            let commit = match self.get_commit_by_hash(&hex::encode(&current_hash)) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Failed to load commit {}: {}", hex::encode(&current_hash), e);
                    break;
                }
            };
            
            for change in commit.changes.iter().rev() {
                if change.table() == table {
                    if let Err(e) = engine.apply_change(change) {
                        eprintln!("Warning: Failed to apply change: {}", e);
                    }
                }
            }
            
            current_hash = commit.parents.get(0).map(|p| p.to_vec()).unwrap_or_default();
        }
        
        Ok(engine.state.get(table).cloned().unwrap_or_default())
    }

    pub fn get_head_hash(&self) -> Result<Vec<u8>> {
        self.db.get(b"HEAD")
            .map_err(|e| BranchDBError::StorageError(e.to_string()))?  // Convert error to string
            .ok_or_else(|| BranchDBError::InvalidInput("No HEAD commit".into()))
    }
}