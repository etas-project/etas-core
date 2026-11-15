module std.text;

import std.text.{TextError, ParseFailure};

public flow parse_bool(text: string) -> Result<bool, TextError> ![] {
    if text == "true" { return Ok(true); }
    if text == "false" { return Ok(false); }
    return Err(ParseFailure);
}

public flow from_bool(value: bool) -> string ![] {
    if value { return "true"; }
    return "false";
}
