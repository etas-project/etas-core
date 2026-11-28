module std.http.codec;

import std.http.codec.HttpCodecError.{MalformedMessage};
import std.http.codec.HttpCodecFailureKind.{ConflictingMessageFraming, ForbiddenResponseBody, InvalidChunkSize, InvalidChunkTerminator, InvalidTrailer, UnexpectedEof};
import std.http.codec.HttpDecodeStep.{Complete, Malformed, NeedMore};

enum BodyPhase { Head, Fixed(usize), UntilEof, Size, Data(usize), Trailers }

public type HttpResponseDecoder = {
    private head_decoder: HttpResponseHeadDecoder,
    private head: Option<HttpWireResponseHead>,
    private phase: BodyPhase,
    private start: usize,
    private scanned: usize,
    private chunk_line: ChunkLine,
    private body: std.bytes.BytesBuilder,
};

impl HttpResponseDecoder {
    public flow new() -> HttpResponseDecoder ![] {
        return HttpResponseDecoder {
            head_decoder= HttpResponseHeadDecoder.new(), head= None, phase= BodyPhase.Head,
            start= 0, scanned= 0, chunk_line= ChunkLine.new(), body= std.bytes.BytesBuilder.new(),
        };
    }

    flow begin_body(var self, head: HttpWireResponseHead, end: usize) -> Result<unit, HttpCodecFailureKind> ![] {
        let (chunked, length) = match head.framing() {
            Ok(value) => value,
            Err(kind) => { return Err(kind); },
        };
        if chunked && length.is_some() { return Err(ConflictingMessageFraming); }
        if (head.status >= 100 && head.status < 200) || head.status == 204 || head.status == 304 {
            if chunked || (head.status != 304 && length.unwrap_or(0) != 0) { return Err(ForbiddenResponseBody); }
            self.phase = BodyPhase.Fixed(0);
        } else if chunked {
            self.phase = BodyPhase.Size;
        } else {
            self.phase = match length { Some(value) => BodyPhase.Fixed(value), None => BodyPhase.UntilEof };
        }
        self.head = Some(head);
        self.start = end;
        self.scanned = end;
        return Ok(());
    }

    // Each retry supplies the same response prefix extended with newly received bytes.
    public flow decode(var self, input: bytes, end_of_stream: bool) -> HttpDecodeStep<HttpWireResponse> ![] {
        let reader = WireReader { input= input, end= input.length };
        while true {
            match self.phase {
                BodyPhase.Head => {
                    var parser = self.head_decoder;
                    let result = parser.decode(input, end_of_stream);
                    self.head_decoder = parser;
                    match result {
                        NeedMore => { return NeedMore; },
                        Malformed(failure) => { return Malformed(failure); },
                        Complete(head, end) => match self.begin_body(head, end) {
                            Ok(_) => {},
                            Err(kind) => { return Malformed(HttpDecodeFailure { kind= kind, offset= end }); },
                        },
                    };
                },
                BodyPhase.Fixed(count) => {
                    if count > input.length - self.start { break; }
                    let end = self.start + count;
                    return Complete(HttpWireResponse { head= self.head.unwrap(), body= input[self.start, end)?.unwrap() }, end);
                },
                BodyPhase.UntilEof => {
                    if !end_of_stream { return NeedMore; }
                    return Complete(HttpWireResponse { head= self.head.unwrap(), body= input[self.start, input.length)?.unwrap() }, input.length);
                },
                BodyPhase.Size => {
                    let (line, next, complete) = match reader.line(self.start, self.scanned) {
                        Complete(view, end) => (view, end, true),
                        Malformed(failure) => { return Malformed(failure); },
                        NeedMore => {
                            self.scanned = reader.resume_offset();
                            (WireView { input= input, begin= self.start, end= self.scanned }, self.scanned, false)
                        },
                    };
                    var parser = self.chunk_line;
                    let result = parser.parse(line, complete);
                    self.chunk_line = parser;
                    match result {
                        Malformed(failure) => {
                            return Malformed(HttpDecodeFailure { kind= failure.kind, offset= self.start + failure.offset });
                        },
                        NeedMore => { break; },
                        Complete(size, _) => {
                            let maximum = usize.maximum();
                            if size.bits > maximum.bits {
                                return Malformed(HttpDecodeFailure { kind= InvalidChunkSize, offset= self.start });
                            }
                            if size.bits > maximum.bits - next.bits {
                                return Malformed(HttpDecodeFailure { kind= InvalidChunkSize, offset= next });
                            }
                            self.start = next;
                            self.scanned = next;
                            self.phase = if size == 0 { BodyPhase.Trailers } else { BodyPhase.Data(usize { bits= size.bits }) };
                        },
                    };
                },
                BodyPhase.Data(count) => {
                    let end = self.start + count;
                    if end > input.length || input.length - end < 2 { break; }
                    if input.get(end) != Some(13) || input.get(end + 1) != Some(10) {
                        return Malformed(HttpDecodeFailure { kind= InvalidChunkTerminator, offset= end });
                    }
                    var body = self.body;
                    body.append(input[self.start, end)?.unwrap());
                    self.body = body;
                    self.start = end + 2;
                    self.scanned = self.start;
                    self.chunk_line = ChunkLine.new();
                    self.phase = BodyPhase.Size;
                },
                BodyPhase.Trailers => {
                    match reader.line(self.start, self.scanned) {
                        Malformed(failure) => { return Malformed(failure); },
                        NeedMore => { self.scanned = reader.resume_offset(); break; },
                        Complete(line, end) => {
                            if line.length() == 0 {
                                return Complete(HttpWireResponse { head= self.head.unwrap(), body= self.body.build() }, end);
                            }
                            let header = match line.header() {
                                Ok(header) => header,
                                Err(_) => { return Malformed(HttpDecodeFailure { kind= InvalidTrailer, offset= self.start }); },
                            };
                            let name = std.text.ascii_lowercase(header.name);
                            if name == "content-length" || name == "transfer-encoding" || name == "trailer" {
                                return Malformed(HttpDecodeFailure { kind= InvalidTrailer, offset= self.start });
                            }
                            self.start = end;
                            self.scanned = end;
                        },
                    };
                },
            };
        }
        if end_of_stream { return Malformed(HttpDecodeFailure { kind= UnexpectedEof, offset= input.length }); }
        return NeedMore;
    }
}

public flow decode_response_incremental(input: bytes, end_of_stream: bool)
    -> HttpDecodeStep<HttpWireResponse> ![] {
    var decoder = HttpResponseDecoder.new();
    return decoder.decode(input, end_of_stream);
}

public flow decode_response(input: bytes) -> Result<HttpWireResponse, HttpCodecError> ![] {
    return match decode_response_incremental(input, true) {
        Complete(response, _) => Ok(response),
        NeedMore => Err(MalformedMessage),
        Malformed(_) => Err(MalformedMessage),
    };
}
