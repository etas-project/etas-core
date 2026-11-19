module std.http.codec;

import std.http.codec.HttpCodecError.{MalformedMessage};
import std.http.codec.HttpCodecFailureKind.{ConflictingContentLength, InvalidContentLength, InvalidHeader, LimitExceeded, UnexpectedEof, UnsupportedTransferEncoding};
import std.http.codec.HttpDecodeStep.{Complete, Malformed, NeedMore};

public flow decode_response_head_prefix(input: bytes, limit: usize) -> HttpDecodeStep<HttpWireResponseHead> ![] {
    let end = if input.length < limit { input.length } else { limit };
    let reader = WireReader { input= input, end= end };
    let (line, next) = match reader.line(0) {
        Complete(line, next) => (line, next),
        Malformed(failure) => { return Malformed(failure); },
        NeedMore => {
            let partial = WireView { input= input, begin= 0, end= end }.partial();
            match partial.status_prefix(false) {
                Some(failure) => { return Malformed(failure); },
                None => {},
            };
            if input.length >= limit {
                return Malformed(HttpDecodeFailure { kind= LimitExceeded, offset= limit });
            }
            return NeedMore;
        },
    };
    match line.status_prefix(true) {
        Some(failure) => { return Malformed(failure); },
        None => {},
    };
    let (version, status, reason) = match line.status() {
        Ok(value) => value,
        Err(failure) => { return Malformed(failure); },
    };
    var offset = next;
    var headers: List<HttpHeader> = [];
    while true limit Iterations(18446744073709551615) {
        match reader.line(offset) {
            Complete(line, next) => {
                if line.length() == 0 {
                    return Complete(HttpWireResponseHead {
                        version= version, status= status, reason= reason, headers= headers.reverse(),
                    }, next);
                }
                let header = match line.header() {
                    Ok(header) => header,
                    Err(kind) => { return Malformed(HttpDecodeFailure { kind= kind, offset= offset }); },
                };
                headers = header :: headers;
                offset = next;
            },
            Malformed(failure) => { return Malformed(failure); },
            NeedMore => {
                let partial = WireView { input= input, begin= offset, end= end }.partial();
                match partial.header_prefix() {
                    Some(bad) => { return Malformed(HttpDecodeFailure { kind= InvalidHeader, offset= offset + bad }); },
                    None => {},
                };
                if input.length >= limit {
                    return Malformed(HttpDecodeFailure { kind= LimitExceeded, offset= limit });
                }
                return NeedMore;
            },
        };
    }
}

public flow decode_response_head_incremental(input: bytes, end_of_stream: bool)
    -> HttpDecodeStep<HttpWireResponseHead> ![] {
    let reader = WireReader { input= input, end= input.length };
    let head_end = match reader.head_end() {
        Complete(_, end) => end,
        Malformed(failure) => { return Malformed(failure); },
        NeedMore => {
            if end_of_stream {
                return Malformed(HttpDecodeFailure { kind= UnexpectedEof, offset= input.length });
            }
            return NeedMore;
        },
    };
    let bounded = WireReader { input= input, end= head_end };
    let (line, next) = match bounded.line(0) {
        Complete(line, next) => (line, next),
        Malformed(failure) => { return Malformed(failure); },
        NeedMore => std.core.abort("complete HTTP head lacks its status line"),
    };
    let (version, status, reason) = match line.status() {
        Ok(value) => value,
        Err(failure) => { return Malformed(failure); },
    };
    var headers: List<HttpHeader> = [];
    var offset = next;
    while true limit Iterations(18446744073709551615) {
        match bounded.line(offset) {
            Complete(line, next) => {
                if line.length() == 0 {
                    return Complete(HttpWireResponseHead {
                        version= version, status= status, reason= reason, headers= headers.reverse(),
                    }, next);
                }
                let header = match line.header() {
                    Ok(value) => value,
                    Err(kind) => { return Malformed(HttpDecodeFailure { kind= kind, offset= offset }); },
                };
                headers = header :: headers;
                offset = next;
            },
            Malformed(failure) => { return Malformed(failure); },
            NeedMore => std.core.abort("complete HTTP head lacks its terminator"),
        };
    }
}

public flow decode_response_head(input: bytes) -> Result<HttpWireResponseHead, HttpCodecError> ![] {
    return match decode_response_head_incremental(input, true) {
        Complete(head, _) => Ok(head),
        NeedMore => Err(MalformedMessage),
        Malformed(_) => Err(MalformedMessage),
    };
}

impl HttpWireResponseHead {
    flow framing(self) -> Result<(bool, Option<usize>), HttpCodecFailureKind> ![] {
        var transfer: usize = 0;
        for header in self.headers limit Iterations(18446744073709551615) {
            if std.text.eq_ignore_ascii_case(header.name, "transfer-encoding") {
                for spelling in std.text.split(header.value, ",") limit Iterations(18446744073709551615) {
                    let encoding = std.text.trim(spelling);
                    if encoding == "" { continue; }
                    if transfer != 0 || !std.text.eq_ignore_ascii_case(encoding, "chunked") {
                        return Err(UnsupportedTransferEncoding);
                    }
                    transfer = transfer + 1;
                }
            }
        }
        var length: Option<usize> = None;
        for header in self.headers limit Iterations(18446744073709551615) {
            if std.text.eq_ignore_ascii_case(header.name, "content-length") {
                for spelling in std.text.split(header.value, ",") limit Iterations(18446744073709551615) {
                    let text = std.text.trim(spelling);
                    if text == "" { return Err(InvalidContentLength); }
                    for scalar in std.text.chars(text) limit Iterations(18446744073709551615) {
                        if !scalar.is_ascii_digit() { return Err(InvalidContentLength); }
                    }
                    let parsed = match std.text.parse_usize(text) {
                        Ok(value) => value,
                        Err(_) => { return Err(InvalidContentLength); },
                    };
                    match length {
                        Some(previous) => {
                            if previous != parsed { return Err(ConflictingContentLength); }
                        },
                        None => {},
                    };
                    length = Some(parsed);
                }
            }
        }
        return Ok((transfer != 0, length));
    }
}
