use serde::{Serialize, Deserialize};
use crate::error::{GitDBError, Result};
use std::collections::HashMap;
use crate::core::models::Change;

pub type TableState = HashMap<String, CrdtValue>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CrdtValue {
    Counter(u64),
    Register(Vec<u8>),
}

#[derive(Debug, Clone)]
pub struct CrdtEngine {
    pub state: HashMap<String, TableState>,
}

impl CrdtEngine {
    pub fn new() -> Self {
        Self {
            state: HashMap::new(),
        }
    }

    pub fn apply_change(&mut self, change: &Change) -> Result<()> {
        match change {
            Change::Insert { table, id, value } => {
                let row = self.state.entry(table.clone()).or_default();
                let decoded: CrdtValue = bincode::deserialize(value)?;
                row.insert(id.clone(), decoded);
            }
            Change::Update { table, id, value } => {
                let row = self.state.entry(table.clone()).or_default();
                let decoded: CrdtValue = bincode::deserialize(value)?;
                row.insert(id.clone(), decoded);
            }
            Change::Delete { table, id } => {
                if let Some(row_map) = self.state.get_mut(table) {
                    row_map.remove(id);
                }
            }
        }
        Ok(())
    }

    pub fn merge(&mut self, other: &Self) -> Result<()> {
        for (table, rows) in &other.state {
            let my_rows = self.state.entry(table.clone()).or_default();
            for (id, val) in rows {
                match (my_rows.get_mut(id), val) {
                    (Some(CrdtValue::Counter(local)), CrdtValue::Counter(remote)) => {
                        *local = (*local).max(*remote);
                    }
                    (Some(CrdtValue::Register(local)), CrdtValue::Register(remote)) => {
                        if *remote > *local {
                            *local = remote.clone();
                        }
                    }
                    (None, val) => {
                        my_rows.insert(id.clone(), val.clone());
                    }
                    // Type mismatch
                    _ => {
                        return Err(GitDBError::TypeMismatch(format!("Type mismatch on merge for ID: {}", id)));
                    }
                }
            }
        }
        Ok(())
    }

    pub fn into_data(self) -> HashMap<String, TableState> {
        self.state
    }
}
