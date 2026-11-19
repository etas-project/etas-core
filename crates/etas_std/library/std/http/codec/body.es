module std.http.codec;

import std.http.codec.HttpCodecError.{MalformedMessage};
import std.http.codec.HttpCodecFailureKind.{ConflictingMessageFraming, ForbiddenResponseBody, InvalidChunkSize, InvalidChunkTerminator, InvalidTrailer, UnexpectedEof};
import std.http.codec.HttpDecodeStep.{Complete, Malformed, NeedMore};

public flow decode_response_incremental(input: bytes, end_of_stream: bool)
    -> HttpDecodeStep<HttpWireResponse> ![] {
    let (head, head_end) = match decode_response_head_incremental(input, end_of_stream) {
        Complete(head, consumed) => (head, consumed),
        NeedMore => { return NeedMore; },
        Malformed(failure) => { return Malformed(failure); },
    };
    let (chunked, length) = match head.framing() {
        Ok(value) => value,
        Err(kind) => { return Malformed(HttpDecodeFailure { kind= kind, offset= head_end }); },
    };
    if chunked && length.is_some() {
        return Malformed(HttpDecodeFailure { kind= ConflictingMessageFraming, offset= head_end });
    }
    let no_body = (head.status >= 100 && head.status < 200) || head.status == 204 || head.status == 304;
    if no_body {
        let forbidden_length = match length {
            Some(value) => head.status != 304 && value != 0,
            None => false,
        };
        if chunked || forbidden_length {
            return Malformed(HttpDecodeFailure { kind= ForbiddenResponseBody, offset= head_end });
        }
        return Complete(HttpWireResponse { head= head, body= bytes { data= [] } }, head_end);
    }
    if chunked {
        let reader = WireReader { input= input, end= input.length };
        var offset = head_end;
        var output: Array<u8> = [];
        var maximum_index: u128 = 0;
        var bit: usize = 0;
        while bit < usize.width() limit Iterations(128) {
            maximum_index = maximum_index * 2 + 1;
            bit = bit + 1;
        }
        while true limit Iterations(18446744073709551615) {
            let line_start = offset;
            let (line, next) = match reader.line(offset) {
                Complete(line, next) => (line, next),
                Malformed(failure) => { return Malformed(failure); },
                NeedMore => {
                    // Even incomplete lines can already contain an invalid size
                    // or extension. Preserve the prefix decoder's diagnosis.
                    let partial = WireView { input= input, begin= offset, end= input.length }.partial();
                    var parser = ChunkLine { input= partial, complete= false, offset= 0 };
                    match parser.parse() {
                        Malformed(failure) => {
                            return Malformed(HttpDecodeFailure {
                                kind= failure.kind, offset= line_start + failure.offset,
                            });
                        },
                        _ => {},
                    };
                    if end_of_stream {
                        return Malformed(HttpDecodeFailure { kind= UnexpectedEof, offset= input.length });
                    }
                    return NeedMore;
                },
            };
            var parser = ChunkLine { input= line, complete= true, offset= 0 };
            let size = match parser.parse() {
                Complete(value, _) => value,
                Malformed(failure) => {
                    return Malformed(HttpDecodeFailure { kind= failure.kind, offset= line_start + failure.offset });
                },
                NeedMore => std.core.abort("complete chunk line has no decode result"),
            };
            if size.bits > maximum_index {
                return Malformed(HttpDecodeFailure { kind= InvalidChunkSize, offset= line_start });
            }
            offset = next;
            if size == 0 {
                while true limit Iterations(18446744073709551615) {
                    let trailer_start = offset;
                    let (trailer, next) = match reader.line(offset) {
                        Complete(line, next) => (line, next),
                        Malformed(failure) => { return Malformed(failure); },
                        NeedMore => {
                            if end_of_stream {
                                return Malformed(HttpDecodeFailure { kind= UnexpectedEof, offset= input.length });
                            }
                            return NeedMore;
                        },
                    };
                    offset = next;
                    if trailer.length() == 0 {
                        return Complete(HttpWireResponse { head= head, body= bytes { data= output } }, offset);
                    }
                    let header = match trailer.header() {
                        Ok(header) => header,
                        Err(_) => { return Malformed(HttpDecodeFailure { kind= InvalidTrailer, offset= trailer_start }); },
                    };
                    let name = std.text.ascii_lowercase(header.name);
                    if name == "content-length" || name == "transfer-encoding" || name == "trailer" {
                        return Malformed(HttpDecodeFailure { kind= InvalidTrailer, offset= trailer_start });
                    }
                }
            }
            let count = usize { bits= size.bits };
            if count > input.length - offset {
                if count.bits > maximum_index - offset.bits {
                    return Malformed(HttpDecodeFailure { kind= InvalidChunkSize, offset= offset });
                }
                if end_of_stream {
                    return Malformed(HttpDecodeFailure { kind= UnexpectedEof, offset= input.length });
                }
                return NeedMore;
            }
            let data_end = offset + count;
            if input.length - data_end < 2 {
                if end_of_stream {
                    return Malformed(HttpDecodeFailure { kind= UnexpectedEof, offset= input.length });
                }
                return NeedMore;
            }
            if input.get(data_end) != Some(13) || input.get(data_end + 1) != Some(10) {
                return Malformed(HttpDecodeFailure { kind= InvalidChunkTerminator, offset= data_end });
            }
            while offset < data_end limit Iterations(18446744073709551615) {
                output = output.push(input.get(offset).unwrap());
                offset = offset + 1;
            }
            offset = data_end + 2;
        }
    }
    match length {
        Some(length) => {
            if length > input.length - head_end {
                if end_of_stream {
                    return Malformed(HttpDecodeFailure { kind= UnexpectedEof, offset= input.length });
                }
                return NeedMore;
            }
            let end = head_end + length;
            let body = WireView { input= input, begin= head_end, end= end }.to_bytes();
            return Complete(HttpWireResponse { head= head, body= body }, end);
        },
        None => {},
    };
    if !end_of_stream { return NeedMore; }
    let body = WireView { input= input, begin= head_end, end= input.length }.to_bytes();
    return Complete(HttpWireResponse { head= head, body= body }, input.length);
}

public flow decode_response(input: bytes) -> Result<HttpWireResponse, HttpCodecError> ![] {
    return match decode_response_incremental(input, true) {
        Complete(response, _) => Ok(response),
        NeedMore => Err(MalformedMessage),
        Malformed(_) => Err(MalformedMessage),
    };
}
