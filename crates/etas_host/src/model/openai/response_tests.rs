use super::*;

#[test]
fn null_optional_tool_fields_are_absent_not_malformed() {
    for message in [
        json!({"content":"text"}),
        json!({"content":"text","tool_calls":null}),
        json!({"content":"text","function_call":null}),
        json!({"content":"text","tool_calls":null,"function_call":null}),
        json!({"content":"text","tool_calls":[],"function_call":null}),
    ] {
        let (content, tools) = decode_openai_tool_calls(&message).unwrap();
        assert!(content.is_empty());
        assert!(tools.is_empty());
    }
    let (_, tools) = decode_openai_tool_calls(&json!({"tool_calls":null,"function_call":{
        "name":"echo","arguments":"{\"value\":90}"
    }}))
    .unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].tool, "echo");
}

#[test]
fn malformed_or_conflicting_nonnull_tool_fields_are_rejected() {
    for message in [
        json!({"tool_calls":{}}),
        json!({"tool_calls":"bad"}),
        json!({"tool_calls":false}),
        json!({"function_call":false}),
        json!({"function_call":{}}),
        json!({"tool_calls":[],"function_call":{"name":"echo","arguments":"{}"}}),
    ] {
        assert!(decode_openai_tool_calls(&message).is_err(), "{message}");
    }
}

fn response(reason: Option<Value>, message: Value) -> ModelResponse {
    let mut choice = json!({"message":message});
    if let Some(reason) = reason {
        choice["finish_reason"] = reason;
    }
    decode_openai_chat_response(
        crate::HostRequestId(49),
        &json!({
            "choices":[choice], "usage":{"prompt_tokens":10,"completion_tokens":4}
        })
        .to_string(),
    )
    .unwrap()
}

#[test]
fn length_retains_usage_and_rejects_even_valid_text_or_truncated_tools() {
    for message in [
        json!({"content":"[\"delegate\",\"Read context.txt\"]","reasoning_content":"PRIVATE_REASONING"}),
        json!({"content":""}),
        json!({"tool_calls":[{"function":{"arguments":"{"}}]}),
    ] {
        let response = response(Some(json!("length")), message);
        assert_eq!(response.id, crate::HostRequestId(49));
        let usage = response.usage.unwrap();
        assert_eq!(usage.input_tokens + usage.output_tokens, 14);
        let error = response.result.unwrap_err();
        assert_eq!(error.code, HostErrorCode::IncompleteResponse);
        assert!(!error.to_string().contains("PRIVATE_REASONING"));
    }
}

#[test]
fn missing_null_unknown_and_conflicting_finish_reasons_fail_closed_with_usage() {
    for reason in [
        None,
        Some(Value::Null),
        Some(json!(17)),
        Some(json!("PRIVATE_UNKNOWN_REASON")),
    ] {
        let response = response(reason, json!({"content":"valid"}));
        assert!(response.usage.is_some());
        let error = response.result.unwrap_err();
        assert_eq!(error.code, HostErrorCode::InvalidResponse);
        assert!(!error.to_string().contains("PRIVATE_UNKNOWN_REASON"));
    }
    assert_eq!(
        response(Some(json!("content_filter")), json!({"content":"valid"}))
            .result
            .unwrap_err()
            .code,
        HostErrorCode::ProviderRejected
    );
    assert!(
        response(
            Some(json!("stop")),
            json!({"content":"valid","tool_calls":null,"function_call":null})
        )
        .result
        .is_ok()
    );
    assert!(
        response(Some(json!("tool_calls")), json!({"content":"valid"}))
            .result
            .is_err()
    );
    assert!(
        response(
            Some(json!("stop")),
            json!({"tool_calls":[{"id":"call","function":{"name":"echo","arguments":"{}"}}]})
        )
        .result
        .is_err()
    );
}
