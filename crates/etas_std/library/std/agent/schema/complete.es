module std.agent.schema;

public type ModelResponse = {
    request_id: u64,
    usage: Option<ModelUsage>,
    result: Result<ModelOutput, ModelError>,
};

public type ModelOutput = {
    message: ModelMessage,
    termination: ModelTermination,
    provider_request_id: Option<string>,
};

public type ModelMessage = {
    provider_data: Array<(string, std.runtime.value.Data)>,
    role: ModelRole,
    content: Array<ModelContent>,
    tool_calls: Array<ModelToolCall>,
    tool_call_id: Option<string>,
};

public enum ModelRole { System, User, Assistant, Tool, }
public enum ModelContent { Text(string), Value(std.runtime.value.Data), }

public type ModelToolCall = {
    id: string,
    tool: string,
    arguments: std.runtime.value.Data,
};

public enum ModelTermination {
    Complete,
    Truncated,
    Refused(string),
    InvalidToolCall(string),
    InvalidOutput(string),
}

public type ModelUsage = {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cost: Option<ModelCostUsage>,
};

public type ModelCostUsage = { micros: u128, currency: string };

public type ModelError = { code: string, message: string };

// Decoding is an ordinary typed source callable. Schema data is the recursive
// JsonValue tree, not a Rust-backed type descriptor masquerading as a value.
public type ResponseDecode<T> = {
    decode: (ModelResponse) -> Result<T, std.runtime.error.SchemaError> ![],
};

public type Schema<T> = {
    definition: std.json.JsonValue,
    decoder: ResponseDecode<T>,
};
