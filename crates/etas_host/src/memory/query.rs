use crate::{HostError, HostErrorCode, HostValue};

pub(super) fn memory_query_matches(
    key: &HostValue,
    value: &HostValue,
    predicate: Option<&HostValue>,
) -> bool {
    let Some(predicate) = predicate else {
        return true;
    };
    match predicate {
        HostValue::String(text) => {
            matches!(key, HostValue::String(key_text) if key_text.contains(text))
                || matches!(value, HostValue::String(value_text) if value_text.contains(text))
        }
        other => key == other || value == other,
    }
}

pub(super) fn validate_query_embedding(embedding: &[f32]) -> Result<(), HostError> {
    if embedding.is_empty()
        || embedding.len() > 4096
        || embedding.iter().any(|value| !value.is_finite())
    {
        return Err(HostError::new(
            HostErrorCode::InvalidRequest,
            "memory vector search requires a non-empty finite embedding",
        ));
    }
    Ok(())
}

pub(super) fn extract_embedding(value: &HostValue) -> Option<&[HostValue]> {
    match value {
        HostValue::List(values) => Some(values),
        HostValue::Record(fields) => fields.iter().find_map(|(name, value)| {
            (name == "embedding").then_some(value).and_then(|value| {
                if let HostValue::List(values) = value {
                    Some(values.as_slice())
                } else {
                    None
                }
            })
        }),
        _ => None,
    }
}

pub(super) fn cosine_similarity(query: &[f32], candidate: &[HostValue]) -> Option<f64> {
    if query.len() != candidate.len() {
        return None;
    }
    let mut dot = 0.0f64;
    let mut query_norm = 0.0f64;
    let mut candidate_norm = 0.0f64;
    for (left, right) in query.iter().copied().zip(candidate.iter()) {
        let right = f64::from(host_value_to_f32(right)?);
        let left = f64::from(left);
        dot += left * right;
        query_norm += left * left;
        candidate_norm += right * right;
    }
    if query_norm == 0.0 || candidate_norm == 0.0 {
        return None;
    }
    Some(dot / (query_norm.sqrt() * candidate_norm.sqrt()))
}

pub(super) fn host_value_to_f32(value: &HostValue) -> Option<f32> {
    match value {
        HostValue::Float(value) if value.is_finite() && (*value as f32).is_finite() => {
            Some(*value as f32)
        }
        HostValue::Int(value) => Some(*value as f32),
        HostValue::UInt(value) => Some(*value as f32),
        _ => None,
    }
}
