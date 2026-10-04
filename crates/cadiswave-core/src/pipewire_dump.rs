//! Decode complete PipeWire JSON batches before publishing a graph.
use crate::model::{OperationError, Result};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};

pub fn parse(bytes: &[u8]) -> Result<Value> {
    let mut batches = serde_json::Deserializer::from_slice(bytes).into_iter::<Vec<Value>>();
    let first = batches
        .next()
        .ok_or_else(|| OperationError::unavailable("Empty pw-dump output"))??;
    let Some(second) = batches.next() else {
        // Consumers validate a single snapshot, including duplicate identities.
        return Ok(Value::Array(first));
    };
    let mut objects = BTreeMap::new();
    apply(&mut objects, first)?;
    apply(&mut objects, second?)?;
    for batch in batches {
        apply(&mut objects, batch?)?;
    }
    Ok(Value::Array(objects.into_values().collect()))
}
fn apply(objects: &mut BTreeMap<u64, Value>, batch: Vec<Value>) -> Result<()> {
    let mut seen = HashSet::new();
    for object in batch {
        let id = object["id"]
            .as_u64()
            .ok_or_else(|| OperationError::unavailable("PipeWire object lacks an ID"))?;
        if !seen.insert(id) {
            return Err(OperationError::unavailable(
                "Duplicate PipeWire batch identity",
            ));
        }
        if object["type"].is_string() {
            objects.insert(id, object);
        } else if object.as_object().is_some_and(|fields| {
            fields.iter().all(|(key, value)| {
                key == "id" || (matches!(key.as_str(), "info" | "props") && value.is_null())
            })
        }) {
            objects.remove(&id);
        } else {
            return Err(OperationError::unavailable(
                "Invalid PipeWire removal record",
            ));
        }
    }
    Ok(())
}
