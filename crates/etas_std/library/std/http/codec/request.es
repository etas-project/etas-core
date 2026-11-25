module std.http.codec;

import std.http.codec.HttpCodecError.{MalformedMessage};

public flow encode_request(request: HttpWireRequest) -> Result<bytes, HttpCodecError> ![] {
    if request.method == "" { return Err(MalformedMessage); }
    for byte in request.method.utf8 {
        if !byte.is_http_token() { return Err(MalformedMessage); }
    }
    if request.version != "HTTP/1.0" && request.version != "HTTP/1.1" {
        return Err(MalformedMessage);
    }
    for scalar in std.text.chars(request.target) {
        if scalar == '\r' || scalar == '\n' { return Err(MalformedMessage); }
    }
    var text = request.method + " " + request.target + " " + request.version + "\r\n";
    var has_content_length = false;
    for header in request.headers {
        if header.name == "" { return Err(MalformedMessage); }
        for byte in header.name.utf8 {
            if !byte.is_http_token() { return Err(MalformedMessage); }
        }
        for scalar in std.text.chars(header.value) {
            if scalar == '\r' || scalar == '\n' { return Err(MalformedMessage); }
        }
        if std.text.eq_ignore_ascii_case(header.name, "content-length") {
            has_content_length = true;
        }
        text = text + header.name + ": " + header.value + "\r\n";
    }
    if request.body.length != 0 && !has_content_length {
        text = text + "content-length: " + std.text.from_usize(request.body.length) + "\r\n";
    }
    text = text + "\r\n";
    return Ok(text.utf8 + request.body);
}
