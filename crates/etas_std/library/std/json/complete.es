module std.json;

// Public declarations migrated from the legacy API inventory.
// Maintained as Etas source; native bindings retain existing execution during migration.

// Structured JSON parse/stringify error support type.
public enum JsonError {
    InvalidJson(string),
}

// Structured JSON value support type.
@std.role("std.json.JsonValue")
public enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(string),
    Array(std.collections.Array<JsonValue>),
    Object(std.collections.Map<string, JsonValue>),
}

// Parse JSON into a structured value or return a JSON error.
@std.binding("std.json.parse")
public flow parse(arg0: string) -> std.result.Result<std.json.JsonValue, std.json.JsonError> ![] {}

// Stringify a structured JSON value or return a JSON error.
@std.binding("std.json.stringify")
public flow stringify(arg0: std.json.JsonValue) -> std.result.Result<string, std.json.JsonError> ![] {}
