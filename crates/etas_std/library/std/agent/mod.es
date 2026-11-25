module std.agent;

import std.json.JsonValue;
import std.agent.session.Conversation;
import std.agent.schema.Schema;

public spec Active;
public spec Autonomous ~ Active {
    flow main(var self) -> unit;
}

public type StepResult<C, O> = {
    turn: AssistantTurn,
    decision: StepDecision<C, O>,
    usage: Option<Usage>,
}

public enum StepDecision<C, O> {
    Final(O),
    CallTools(Array<ToolCall<C>>),
    Stopped(StopReason),
}

public type AssistantTurn = {
    request_id: Option<string>,
    content: Array<MessageContent>,
    provider_data: Map<string, JsonValue>,
}

public enum MessageContent {
    Text(string),
    Value(JsonValue),
}

public type ToolRef = {
    id: string,
    revision: string,
}

public type ToolCall<C> = {
    id: string,
    tool: ToolRef,
    args: JsonValue,
}

public type ToolResult<C> = {
    call_id: string,
    result: Result<JsonValue, StepDiagnostic>,
}

public enum StopReason {
    Truncated,
    Refused(string),
    InvalidOutput(StepDiagnostic),
    InvalidToolCall(StepDiagnostic),
}

public type StepDiagnostic = {
    code: string,
    message: string,
    path: Option<string>,
}

public type Usage = {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cost: Option<{ micros: u128, currency: string }>,
}

// C is the checked agent-method context. The action must preserve its
// latent tool/error/trace contract; transparent protocol records grant no authority.

public flow step<C, O>(conversation: Conversation, schema: Schema<O>) -> StepResult<C, O> ![] {
    return perform std.effects.Agentic.step<C, O>(conversation, schema);
}
