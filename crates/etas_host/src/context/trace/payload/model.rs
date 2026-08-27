use crate::{
    HostTraceFieldSensitivity, HostTracePayload, HostValue, ModelContent, ModelMessage,
    ModelRequest, ModelRole, ModelToolCall, ModelToolChoice, ToolRequest,
};

use super::{HostTraceRequest, option, record, schema, tool_ref, tool_schema};

impl HostTraceRequest for ModelRequest {
    fn trace_payload(&self) -> HostTracePayload {
        HostTracePayload::new("model", "Agentic.infer")
            .with_field(
                "provider",
                option(
                    self.provider
                        .as_ref()
                        .map(|provider| HostValue::String(provider.0.clone())),
                ),
                HostTraceFieldSensitivity::Public,
            )
            .with_field(
                "model",
                HostValue::String(self.model.0.clone()),
                HostTraceFieldSensitivity::Public,
            )
            .with_field(
                "messages",
                HostValue::List(self.messages.iter().map(message).collect()),
                HostTraceFieldSensitivity::Sensitive,
            )
            .with_field(
                "tools",
                HostValue::List(self.tools.iter().map(tool_schema).collect()),
                HostTraceFieldSensitivity::Public,
            )
            .with_field(
                "tool_choice",
                tool_choice(&self.tool_choice),
                HostTraceFieldSensitivity::Public,
            )
            .with_field(
                "response_schema",
                option(self.response_schema.as_ref().map(schema)),
                HostTraceFieldSensitivity::Public,
            )
            .with_field(
                "policy_ref",
                option(self.policy_ref.clone()),
                HostTraceFieldSensitivity::Sensitive,
            )
            .with_field(
                "options",
                record([
                    (
                        "temperature",
                        option(
                            self.options
                                .temperature
                                .map(|value| HostValue::Float(value as f64)),
                        ),
                    ),
                    (
                        "max_output_tokens",
                        option(
                            self.options
                                .max_output_tokens
                                .map(|value| HostValue::UInt(value as u128)),
                        ),
                    ),
                    ("metadata", HostValue::Record(self.options.metadata.clone())),
                ]),
                HostTraceFieldSensitivity::Sensitive,
            )
    }
}

impl HostTraceRequest for ToolRequest {
    fn trace_payload(&self) -> HostTracePayload {
        HostTracePayload::new("tool", "Tool.invoke")
            .with_field(
                "tool",
                tool_ref(&self.tool),
                HostTraceFieldSensitivity::Public,
            )
            .with_field(
                "args",
                self.args.clone(),
                HostTraceFieldSensitivity::Sensitive,
            )
    }
}

fn message(message: &ModelMessage) -> HostValue {
    record([
        (
            "role",
            HostValue::String(role_name(message.role).to_owned()),
        ),
        (
            "content",
            HostValue::List(message.content.iter().map(content).collect()),
        ),
        (
            "tool_call_id",
            option(message.tool_call_id.clone().map(HostValue::String)),
        ),
        (
            "tool_calls",
            HostValue::List(message.tool_calls.iter().map(tool_call).collect()),
        ),
    ])
}

fn role_name(role: ModelRole) -> &'static str {
    match role {
        ModelRole::System => "system",
        ModelRole::User => "user",
        ModelRole::Assistant => "assistant",
        ModelRole::Tool => "tool",
    }
}

fn content(content: &ModelContent) -> HostValue {
    match content {
        ModelContent::Text(text) => super::variant("Text", vec![HostValue::String(text.clone())]),
        ModelContent::Value(value) => super::variant("Value", vec![value.clone()]),
    }
}

fn tool_call(call: &ModelToolCall) -> HostValue {
    record([
        ("id", HostValue::String(call.id.clone())),
        ("tool", HostValue::String(call.tool.clone())),
        ("args", call.args.clone()),
    ])
}

fn tool_choice(choice: &ModelToolChoice) -> HostValue {
    match choice {
        ModelToolChoice::Auto => super::variant("Auto", Vec::new()),
        ModelToolChoice::RequiredAny => super::variant("RequiredAny", Vec::new()),
        ModelToolChoice::RequiredTool(tool) => {
            super::variant("RequiredTool", vec![HostValue::String(tool.clone())])
        }
    }
}
