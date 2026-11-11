use crate::{HostError, HostErrorCode, ModelUsage};
use serde_json::Value;

pub(super) fn decode(
    value: &Value,
    input: &str,
    output: &str,
) -> Result<Option<ModelUsage>, HostError> {
    let Some(usage) = value.get("usage").filter(|usage| !usage.is_null()) else {
        return Ok(None);
    };
    let usage = usage.as_object().ok_or_else(|| {
        HostError::new(
            HostErrorCode::InvalidResponse,
            "model usage is not an object",
        )
    })?;
    let token = |name| match usage.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value.as_u64().map(Some).ok_or_else(|| {
            HostError::new(
                HostErrorCode::InvalidResponse,
                "model usage token count is not a u64",
            )
            .with_detail("field", name)
        }),
    };
    Ok(Some(ModelUsage {
        input_tokens: token(input)?,
        output_tokens: token(output)?,
        cost: None,
    }))
}

#[cfg(test)]
mod tests {
    #[test]
    fn partial_usage_keeps_unknown_token_fields() {
        let usage = super::decode(
            &serde_json::json!({"usage":{"output":7}}),
            "input",
            "output",
        )
        .unwrap()
        .unwrap();
        assert_eq!(usage.input_tokens, None);
        assert_eq!(usage.output_tokens, Some(7));
        assert_eq!(usage.cost, None);
    }
}
