use std::collections::HashMap;
use crdts::{CmRDT, CvRDT, GCounter, LWWReg};
use crate::{
    core::models::Change,
    error::{GitDBError, Result},
};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrdtValue {
    Counter(GCounter<u64>),
    Register(LWWReg<Vec<u8>, String>),
}

impl CrdtValue {
    pub fn merge(&mut self, other: &Self) -> Result<()> {
        match (self, other) {
            (CrdtValue::Counter(a), CrdtValue::Counter(b)) => {
                a.merge(b.clone());
                Ok(())
            },
            (CrdtValue::Register(a), CrdtValue::Register(b)) => {
                a.merge(b.clone());
                Ok(())
            },
            _ => Err(GitDBError::TypeMismatch(
                "Cannot merge different CRDT types".to_string()
            )),
        }
    }
}

pub struct CrdtEngine {
    state: HashMap<String, HashMap<Vec<u8>, CrdtValue>>,
}

impl CrdtEngine {
    pub fn new() -> Self {
        Self {
            state: HashMap::new(),
        }
    }

    pub fn apply_change(&mut self, change: &Change) -> Result<()> {
        match change {
            Change::Insert { table, id, data } => {
                let value: CrdtValue = bincode::deserialize(data)?;
                self.state
                    .entry(table.clone())
                    .or_default()
                    .insert(id.clone(), value);
            },
            Change::Update { table, id, data } => {
                let new_value: CrdtValue = bincode::deserialize(data)?;
                if let Some(table_data) = self.state.get_mut(table) {
                    table_data
                        .entry(id.clone())
                        .and_modify(|existing| {
                            existing.merge(&new_value).expect("Type-checked merge");
                        })
                        .or_insert(new_value);
                }
            },
            Change::Delete { table, id } => {
                if let Some(table_data) = self.state.get_mut(table) {
                    table_data.remove(id);
                }
            },
            _ => {} // Ignore other change types for now
        }
        Ok(())
    }

    pub fn get_value(&self, table: &str, id: &[u8]) -> Option<&CrdtValue> {
        self.state.get(table)?.get(id)
    }
}