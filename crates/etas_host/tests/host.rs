use etas_host::{
    ApprovalDecision, ApprovalGrant, AuthorityContext, Budget, HostActionGrant, HostError,
    HostErrorCode, HostRequestId, HostRequestKind, HostSchema, HostValue, HostValueCodec,
    McpToolProtocolAdapter, ModelContent, ModelMessage, ModelName, ModelOptions, ModelRequest,
    ModelResponse, ModelRole, ModelUsage, OpenAiProtocolAdapter, SandboxPolicy, TimeBudget,
    TokenBudget, ToolRef, ToolRequest, ToolResponse, TraceContext, TraceEvent, TraceId,
    TraceSpanId, host_value_to_json,
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct TestValue(String);

struct TestCodec;

impl HostValueCodec<TestValue> for TestCodec {
    type Error = &'static str;

    fn encode(value: &TestValue) -> Result<HostValue, Self::Error> {
        Ok(HostValue::String(value.0.clone()))
    }

    fn decode(value: HostValue) -> Result<TestValue, Self::Error> {
        match value {
            HostValue::String(value) => Ok(TestValue(value)),
            _ => Err("expected string host value"),
        }
    }
}

#[test]
fn host_value_codec_is_engine_owned_and_roundtrips_through_host_value() {
    let value = TestValue("draft".to_owned());
    let encoded = TestCodec::encode(&value).expect("test value should encode");
    assert_eq!(encoded, HostValue::String("draft".to_owned()));
    assert_eq!(
        TestCodec::decode(encoded).expect("test value should decode"),
        value
    );
}

#[test]
fn host_value_json_encoding_rejects_lossy_values() {
    assert_eq!(
        host_value_to_json(&HostValue::Int(i128::MAX))
            .expect_err("out-of-range signed integer should not encode")
            .code,
        HostErrorCode::SchemaMismatch
    );
    assert_eq!(
        host_value_to_json(&HostValue::Float(f64::NAN))
            .expect_err("non-finite float should not encode")
            .code,
        HostErrorCode::SchemaMismatch
    );
    assert_eq!(
        host_value_to_json(&HostValue::Record(vec![
            ("field".to_owned(), HostValue::Bool(true)),
            ("field".to_owned(), HostValue::Bool(false)),
        ]))
        .expect_err("duplicate record fields should not encode as JSON object")
        .code,
        HostErrorCode::SchemaMismatch
    );
}

#[test]
fn model_protocol_adapter_preserves_boundary_context() {
    let authority = AuthorityContext {
        grants: vec![HostActionGrant::allow("Agentic", "infer")],
        approvals: Vec::new(),
        sandbox: SandboxPolicy::deny_all(),
        policy: Default::default(),
    };
    let trace = TraceContext {
        trace_id: TraceId(7),
        parent_span: Some(TraceSpanId(3)),
    };
    let budget = Budget {
        tokens: Some(TokenBudget { max_tokens: 128 }),
        time: Some(TimeBudget { max_millis: 250 }),
        cost: None,
    };
    let request = ModelRequest {
        id: HostRequestId(1),
        provider: None,
        model: ModelName("local-test".to_owned()),
        messages: vec![ModelMessage {
            role: ModelRole::User,
            content: vec![ModelContent::Text("hello".to_owned())],
            tool_call_id: None,
            tool_calls: Vec::new(),
        }],
        tools: Vec::new(),
        tool_choice: Default::default(),
        response_schema: None,
        policy_ref: None,
        options: ModelOptions::default(),
        authority: authority.clone(),
        trace: trace.clone(),
        budget: budget.clone(),
    };

    let adapter = OpenAiProtocolAdapter::local_omlx().expect("local oMLX endpoint should be valid");
    let provider_request = adapter.encode_request(request.clone());
    assert_eq!(
        provider_request.base_url,
        OpenAiProtocolAdapter::LOCAL_OMLX_BASE_URL
    );
    assert_eq!(provider_request.request.id, request.id);
    assert_eq!(provider_request.request.authority, authority);
    assert_eq!(provider_request.request.trace, trace);
    assert_eq!(provider_request.request.budget, budget);

    let response = ModelResponse {
        id: request.id,
        message: ModelMessage {
            role: ModelRole::Assistant,
            content: vec![ModelContent::Text("ok".to_owned())],
            tool_call_id: None,
            tool_calls: Vec::new(),
        },
        tool_calls: Vec::new(),
        usage: Some(ModelUsage {
            input_tokens: 2,
            output_tokens: 1,
        }),
    };
    let decoded = OpenAiProtocolAdapter::decode_response(etas_host::OpenAiProviderResponse {
        response: response.clone(),
    })
    .expect("provider envelope should decode");
    assert_eq!(decoded, response);
}

#[test]
fn tool_protocol_adapter_preserves_request_and_result() {
    let request = ToolRequest {
        id: HostRequestId(4),
        tool: ToolRef::anonymous_test("host.echo"),
        args: HostValue::Record(vec![(
            "message".to_owned(),
            HostValue::String("hi".to_owned()),
        )]),
        authority: AuthorityContext {
            grants: vec![HostActionGrant::allow("Tool", "host.echo")],
            approvals: Vec::new(),
            sandbox: SandboxPolicy::deny_all(),
            policy: Default::default(),
        },
        trace: TraceContext::root(TraceId(9)),
        budget: Budget::default(),
    };

    let adapter = McpToolProtocolAdapter::new("http://example.com")
        .expect("public test endpoint should be valid");
    let envelope = adapter.encode_request(request.clone());
    assert_eq!(envelope.request, request);

    let response = ToolResponse {
        id: request.id,
        result: Ok(HostValue::Bool(true)),
    };
    let decoded = McpToolProtocolAdapter::decode_response(etas_host::McpToolResponseEnvelope {
        response: response.clone(),
    })
    .expect("tool envelope should decode");
    assert_eq!(decoded, response);
}

#[test]
fn authority_trace_and_errors_are_rendering_neutral_values() {
    let grant = ApprovalGrant {
        id: HostRequestId(5),
        grants: vec![HostActionGrant::allow_with_args(
            "File",
            "write",
            vec![etas_host::ActionArgPattern::Prefix(vec![
                "/workspace".to_owned(),
            ])],
        )],
        reason: "approved for test".to_owned(),
    };
    let decision = ApprovalDecision::Approved {
        grant: grant.clone(),
    };
    assert!(matches!(decision, ApprovalDecision::Approved { .. }));

    let error = HostError::new(HostErrorCode::AuthorityDenied, "approval denied")
        .with_detail("grant", "file-write");
    let event = TraceEvent::HostRequestFinished {
        id: HostRequestId(5),
        outcome: etas_host::HostOutcome::Failed(error.clone()),
    };

    assert_eq!(error.details[0].key, "grant");
    assert!(matches!(
        event,
        TraceEvent::HostRequestFinished {
            id: HostRequestId(5),
            ..
        }
    ));
}

#[test]
fn schemas_describe_host_boundaries_without_engine_values() {
    let schema = HostSchema::Record(vec![etas_host::HostFieldSchema {
        name: "message".to_owned(),
        schema: HostSchema::String,
        optional: false,
    }]);

    let HostSchema::Record(fields) = schema else {
        panic!("expected record host schema");
    };
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].name, "message");
    assert_eq!(fields[0].schema, HostSchema::String);
    assert!(!fields[0].optional);

    let event = TraceEvent::HostRequestStarted {
        id: HostRequestId(6),
        kind: HostRequestKind::Tool,
        authority: Box::new(AuthorityContext {
            grants: vec![HostActionGrant::allow("Tool", "host.echo")],
            approvals: Vec::new(),
            sandbox: SandboxPolicy::deny_all(),
            policy: Default::default(),
        }),
    };
    assert!(matches!(
        event,
        TraceEvent::HostRequestStarted {
            kind: HostRequestKind::Tool,
            ..
        }
    ));
}
