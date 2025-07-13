use crate::core::crdt::CrdtEngine;
use crate::core::models::Change;
use crate::error::Result;

pub fn merge_states(state1: &mut CrdtEngine, state2: &CrdtEngine) -> Result<Vec<Change>> {
    let mut changes = Vec::new();

    for (table, rows) in state2.state.iter() {
        let local_rows = state1.state.entry(table.clone()).or_default();

        for (id, value) in rows {
            match local_rows.get(id) {
                Some(local_val) => {
                    if local_val != value {
                        local_rows.insert(id.clone(), value.clone());
                        changes.push(Change::Update {
                            table: table.clone(),
                            id: id.clone(),
                            value: bincode::serialize(value)?,
                        });
                    }
                }
                None => {
                    local_rows.insert(id.clone(), value.clone());
                    changes.push(Change::Insert {
                        table: table.clone(),
                        id: id.clone(),
                        value: bincode::serialize(value)?,
                    });
                }
            }
        }
    }

    Ok(changes)
}