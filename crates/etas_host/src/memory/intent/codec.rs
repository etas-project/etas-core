use super::*;
use std::collections::BTreeMap;

const FORMAT: &str = "etas.memory.write-intent.v1";

pub(super) fn encode(
    intent: &MemoryWriteIntent,
    limits: &StorageLimits,
) -> Result<String, HostError> {
    let (key, value, condition) = match &intent.mutation {
        MemoryMutation::Put {
            key,
            value,
            condition,
        } => (key, Some(value), condition),
        MemoryMutation::Delete { key, condition } => (key, None, condition),
    };
    let condition = match condition {
        WriteCondition::Any => variant("Any", vec![]),
        WriteCondition::Missing => variant("Missing", vec![]),
        WriteCondition::Exists => variant("Exists", vec![]),
        WriteCondition::Match(version) => variant("Match", vec![string(version.as_token())]),
    };
    let payload = match value {
        Some(value) => variant("Put", vec![key.clone(), value.clone(), condition]),
        None => variant("Delete", vec![key.clone(), condition]),
    };
    let envelope = HostValue::Record(vec![
        ("format".into(), string(FORMAT)),
        ("region".into(), string(&intent.store.region.stable_id)),
        (
            "schema".into(),
            match &intent.store.region.schema_fingerprint {
                Some(schema) => string(schema),
                None => HostValue::Unit,
            },
        ),
        (
            "path".into(),
            HostValue::List(intent.store.path.iter().map(|part| string(part)).collect()),
        ),
        ("operation".into(), string(intent.operation.key.as_str())),
        (
            "fingerprint".into(),
            string(&intent.operation.request_fingerprint),
        ),
        ("mutation".into(), payload),
    ]);
    crate::value::tagged::encode_with_limits(&envelope, limits)
}

pub(super) fn decode(
    encoded: &str,
    limits: &StorageLimits,
) -> Result<MemoryWriteIntent, HostError> {
    let HostValue::Record(fields) = crate::value::tagged::decode_with_limits(encoded, limits)?
    else {
        return Err(invalid());
    };
    if fields.len() != 7 {
        return Err(invalid());
    }
    let mut fields = fields.into_iter().collect::<BTreeMap<_, _>>();
    if take_string(&mut fields, "format")? != FORMAT {
        return Err(invalid());
    }
    let region = take_string(&mut fields, "region")?;
    let schema = match fields.remove("schema") {
        Some(HostValue::Unit) => None,
        Some(HostValue::String(value)) => Some(value),
        _ => return Err(invalid()),
    };
    let Some(HostValue::List(path)) = fields.remove("path") else {
        return Err(invalid());
    };
    let path = path
        .into_iter()
        .map(|part| match part {
            HostValue::String(part) => Ok(part),
            _ => Err(invalid()),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let operation = StorageOperationRef {
        key: StorageOperationKey::parse(&take_string(&mut fields, "operation")?)?,
        request_fingerprint: take_string(&mut fields, "fingerprint")?,
    };
    let Some(HostValue::Variant {
        name,
        fields: mut args,
    }) = fields.remove("mutation")
    else {
        return Err(invalid());
    };
    let arity = match name.as_str() {
        "Put" => 3,
        "Delete" => 2,
        _ => return Err(invalid()),
    };
    if args.len() != arity || !fields.is_empty() {
        return Err(invalid());
    }
    let condition = decode_condition(args.pop().ok_or_else(invalid)?)?;
    let mutation = if name == "Put" {
        let value = args.pop().ok_or_else(invalid)?;
        MemoryMutation::Put {
            key: args.pop().ok_or_else(invalid)?,
            value,
            condition,
        }
    } else {
        MemoryMutation::Delete {
            key: args.pop().ok_or_else(invalid)?,
            condition,
        }
    };
    Ok(MemoryWriteIntent {
        store: StoreRef {
            region: crate::MemoryRegionRef {
                stable_id: region,
                schema_fingerprint: schema,
            },
            path,
        },
        mutation,
        operation,
    })
}

fn decode_condition(value: HostValue) -> Result<WriteCondition, HostError> {
    let HostValue::Variant { name, fields } = value else {
        return Err(invalid());
    };
    match (name.as_str(), fields.as_slice()) {
        ("Any", []) => Ok(WriteCondition::Any),
        ("Missing", []) => Ok(WriteCondition::Missing),
        ("Exists", []) => Ok(WriteCondition::Exists),
        ("Match", [HostValue::String(version)]) => {
            Ok(WriteCondition::Match(crate::MemoryVersion::parse(version)?))
        }
        _ => Err(invalid()),
    }
}
fn take_string(fields: &mut BTreeMap<String, HostValue>, key: &str) -> Result<String, HostError> {
    match fields.remove(key) {
        Some(HostValue::String(value)) => Ok(value),
        _ => Err(invalid()),
    }
}
fn string(value: &str) -> HostValue {
    HostValue::String(value.into())
}
fn variant(name: &str, fields: Vec<HostValue>) -> HostValue {
    HostValue::Variant {
        name: name.into(),
        fields,
    }
}
