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
