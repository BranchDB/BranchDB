use serde::{Serialize, Deserialize};
use crate::error::{GitDBError, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CrdtValue {
    Counter(u64),
    Register(Vec<u8>),
}

#[derive(Debug, Clone)]
pub struct CrdtEngine {
    pub state: std::collections::HashMap<String, std::collections::HashMap<String, CrdtValue>>,
}

impl CrdtEngine {
    pub fn new() -> Self {
        Self {
            state: std::collections::HashMap::new(),
        }
    }

    pub fn apply_change(&mut self, change: &crate::core::models::Change) -> Result<()> {
        match change {
            crate::core::models::Change::Insert { table, id, value } |
            crate::core::models::Change::Update { table, id, value } => {
                let table_map = self.state.entry(table.clone()).or_default();
                let deserialized: CrdtValue = bincode::deserialize(value)?;
                table_map.insert(id.clone(), deserialized);
            }
            crate::core::models::Change::Delete { table, id } => {
                if let Some(table_map) = self.state.get_mut(table) {
                    table_map.remove(id);
                }
            }
        }
        Ok(())
    }

    pub fn merge(&mut self, other: &Self) -> Result<()> {
        for (table, rows) in &other.state {
            let entry = self.state.entry(table.clone()).or_default();
            for (id, new_val) in rows {
                match (entry.get_mut(id), new_val) {
                    (Some(CrdtValue::Counter(a)), CrdtValue::Counter(b)) => {
                        *a = (*a).max(*b);
                    }
                    (Some(CrdtValue::Register(a)), CrdtValue::Register(b)) => {
                        if *b > *a {
                            *a = b.clone();
                        }
                    }
                    (None, val) => {
                        entry.insert(id.clone(), val.clone());
                    }
                    _ => {
                        return Err(GitDBError::TypeMismatch("cannot merge different CRDT types".into()));
                    }
                }
            }
        }
        Ok(())
    }

    pub fn into_data(self) -> std::collections::HashMap<String, std::collections::HashMap<String, CrdtValue>> {
        self.state
    }
}
