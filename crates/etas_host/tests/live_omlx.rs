use etas_host::{
    AnthropicProtocolAdapter, AuthConfig, AuthorityContext, Budget, CommandPolicy,
    DestructiveOpPolicy, FilesystemPolicy, HostActionGrant, HostRequestId, ModelClient,
    ModelContent, ModelMessage, ModelName, ModelOptions, ModelRequest, ModelResponse, ModelRole,
    NetworkEndpoint, NetworkPolicy, OpenAiProtocolAdapter, SandboxPolicy, TimeBudget, TokenBudget,
    TraceContext, TraceId,
};

const LIVE_FLAG: &str = "ETAS_HOST_LIVE_OMLX";
const API_KEY_ENV: &str = "ETAS_HOST_OMLX_API_KEY";
const SMALL_LOCAL_MODEL: &str = "Qwen3.5-0.8B-MLX-4bit";

#[tokio::test]
#[ignore = "requires local omlx server, ETAS_HOST_LIVE_OMLX=1, and ETAS_HOST_OMLX_API_KEY"]
async fn local_omlx_openai_adapter_can_complete_when_enabled() {
    if std::env::var(LIVE_FLAG).as_deref() != Ok("1") {
        return;
    }

    assert_eq!(
        OpenAiProtocolAdapter::LOCAL_OMLX_BASE_URL,
        "http://127.0.0.1:8848/v1"
    );
    assert_eq!(SMALL_LOCAL_MODEL, "Qwen3.5-0.8B-MLX-4bit");

    let adapter = OpenAiProtocolAdapter::local_omlx()
        .expect("local oMLX endpoint should be valid")
        .with_auth(live_auth());
    let response = adapter
        .complete(local_completion_request(HostRequestId(1001)))
        .await
        .expect("local omlx OpenAI-compatible completion should return a response");
    assert_non_empty_completion(response);
}

#[tokio::test]
#[ignore = "requires local omlx server, ETAS_HOST_LIVE_OMLX=1, and ETAS_HOST_OMLX_API_KEY"]
async fn local_omlx_anthropic_adapter_can_complete_when_enabled() {
    if std::env::var(LIVE_FLAG).as_deref() != Ok("1") {
        return;
    }

    assert_eq!(
        AnthropicProtocolAdapter::LOCAL_OMLX_BASE_URL,
        "http://127.0.0.1:8848"
    );
    assert_eq!(SMALL_LOCAL_MODEL, "Qwen3.5-0.8B-MLX-4bit");

    let adapter = AnthropicProtocolAdapter::local_omlx()
        .expect("local oMLX endpoint should be valid")
        .with_auth(live_auth());
    let response = adapter
        .complete(local_completion_request(HostRequestId(1002)))
        .await
        .expect("local omlx Anthropic-compatible completion should return a response");
    assert_non_empty_completion(response);
}

fn local_completion_request(id: HostRequestId) -> ModelRequest {
    ModelRequest {
        id,
        provider: None,
        model: ModelName(SMALL_LOCAL_MODEL.to_owned()),
        messages: vec![
            ModelMessage {
                role: ModelRole::System,
                content: vec![ModelContent::Text(
                    "Answer with one short sentence.".to_owned(),
                )],
                tool_call_id: None,
                tool_calls: Vec::new(),
            },
            ModelMessage {
                role: ModelRole::User,
                content: vec![ModelContent::Text("Say hello from Etas.".to_owned())],
                tool_call_id: None,
                tool_calls: Vec::new(),
            },
        ],
        tools: Vec::new(),
        tool_choice: Default::default(),
        response_schema: None,
        policy_ref: None,
        options: ModelOptions {
            temperature: Some(0.0),
            max_output_tokens: Some(1024),
            metadata: Vec::new(),
        },
        authority: AuthorityContext {
            grants: vec![
                HostActionGrant::allow("Agentic", "infer"),
                HostActionGrant::allow_with_args(
                    "Network",
                    "request",
                    vec![etas_host::ActionArgPattern::Exact(
                        etas_host::HostValue::String("127.0.0.1:8848".to_owned()),
                    )],
                ),
            ],
            approvals: Vec::new(),
            sandbox: SandboxPolicy::allow_listed(
                FilesystemPolicy::deny_all(),
                NetworkPolicy::allow_endpoints(vec![NetworkEndpoint::new(
                    "http",
                    "127.0.0.1",
                    8848,
                )]),
                CommandPolicy::deny_all(),
                DestructiveOpPolicy::deny_all(),
            ),
            policy: Default::default(),
        },
        trace: TraceContext::root(TraceId(id.0)),
        budget: Budget {
            tokens: Some(TokenBudget { max_tokens: 64 }),
            time: Some(TimeBudget { max_millis: 2500 }),
            cost: None,
        },
    }
}

fn live_auth() -> AuthConfig {
    AuthConfig::BearerToken(
        std::env::var(API_KEY_ENV)
            .expect("ETAS_HOST_OMLX_API_KEY must be set when live omlx tests are enabled"),
    )
}

fn assert_non_empty_completion(response: ModelResponse) {
    let text = response
        .message
        .content
        .iter()
        .filter_map(|content| match content {
            ModelContent::Text(text) => Some(text.as_str()),
            ModelContent::Value(_) => None,
        })
        .collect::<Vec<_>>()
        .join("");
    assert!(
        !text.trim().is_empty() || !response.tool_calls.is_empty(),
        "live completion should return assistant text or a tool call"
    );
}
