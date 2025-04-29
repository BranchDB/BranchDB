use crate::core::models::Commit;
use crate::core::crdt::CrdtEngine;
use crate::error::{GitDBError, Result};
use rocksdb::DB;
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;
use sqlparser::ast::{Statement, Query, SetExpr};

pub struct QueryProcessor<'a> {
    db: &'a DB,
}

impl<'a> QueryProcessor<'a> {
    pub fn new(db: &'a DB) -> Self {
        QueryProcessor { db }
    }

    pub fn execute(&self, sql: &str) -> Result<()> {
        let dialect = GenericDialect;
        let ast = Parser::parse_sql(&dialect, sql)
            .map_err(|e| GitDBError::InvalidInput(format!("SQL parse error: {}", e)))?;

        if ast.len() != 1 {
            return Err(GitDBError::InvalidInput("Only one SQL statement is allowed".into()));
        }

        let Statement::Query(query) = &ast[0] else {
            return Err(GitDBError::InvalidInput("Only SELECT queries are supported".into()));
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
            return Err(GitDBError::InvalidInput("Expected SELECT statement".into()));
        };

        let from = select.from.get(0)
            .ok_or_else(|| GitDBError::InvalidInput("Missing FROM clause".into()))?;

        let table_name = from.relation.to_string();

        let Some(with) = &query.with else {
            return Err(GitDBError::InvalidInput("Missing WITH clause".into()));
        };

        let cte = with.cte_tables.get(0)
            .ok_or_else(|| GitDBError::InvalidInput("Missing CTE in WITH clause".into()))?;

        let commit_hash = cte.alias.name.to_string();
        Ok((table_name, commit_hash))
    }

    fn get_commit_by_hash(&self, hex_hash: &str) -> Result<Commit> {
        let hash_bytes = hex::decode(hex_hash)
            .map_err(|_| GitDBError::InvalidInput("Invalid hex string for commit hash".into()))?;

        let raw = self.db.get(&hash_bytes).map_err(GitDBError::StorageError)?
            .ok_or_else(|| GitDBError::InvalidInput("Commit not found".into()))?;

        let commit: Commit = bincode::deserialize(&raw)?;
        Ok(commit)
    }
}
