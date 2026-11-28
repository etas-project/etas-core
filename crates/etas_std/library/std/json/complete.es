module std.json;

// Structured JSON parse/stringify error support type.
public enum JsonError {
    InvalidJson(string),
}

// Structured JSON value support type.
public enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(string),
    Array(std.collections.Array<JsonValue>),
    Object(std.collections.Map<string, JsonValue>),
}

public flow parse(text: string) -> Result<JsonValue, JsonError> ![] {
    var reader = JsonReader { input= text.utf8, offset= 0, position= 0 };
    return reader.document()?;
}

public flow stringify(value: JsonValue) -> Result<string, JsonError> ![] {
    let writer = JsonWriter {};
    return writer.value(value)?;
}
