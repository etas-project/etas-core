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
    var input: Array<char> = [];
    for scalar in std.text.chars(text) {
        input = input.push(scalar);
    }
    var reader = JsonReader { input= input, position= 0 };
    return reader.document()?;
}

public flow stringify(value: JsonValue) -> Result<string, JsonError> ![] {
    let writer = JsonWriter {};
    return writer.value(value)?;
}
