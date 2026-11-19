module std.http.codec;

import std.http.codec.HttpCodecFailureKind.{InvalidChunkExtension, InvalidChunkSize, LimitExceeded};
import std.http.codec.HttpDecodeStep.{Complete, Malformed, NeedMore};

type ChunkLine = { private input: WireView, private complete: bool, private offset: usize };

impl ChunkLine {
    flow failure(self, kind: HttpCodecFailureKind, offset: usize, missing: bool) -> HttpDecodeStep<u64> ![] {
        if missing && !self.complete { return NeedMore; }
        return Malformed(HttpDecodeFailure { kind= kind, offset= offset });
    }

    flow whitespace(var self) -> unit ![] {
        while self.offset < self.input.length() limit Iterations(18446744073709551615) {
            let byte = self.input.byte(self.offset);
            if byte != 32 && byte != 9 { return; }
            self.offset = self.offset + 1;
        }
    }

    flow token(var self) -> bool ![] {
        let start = self.offset;
        while self.offset < self.input.length() && self.input.byte(self.offset).is_http_token()
            limit Iterations(18446744073709551615) {
            self.offset = self.offset + 1;
        }
        return self.offset != start;
    }

    flow parse(var self) -> HttpDecodeStep<u64> ![] {
        var size: u64 = 0;
        while self.offset < self.input.length() limit Iterations(18446744073709551615) {
            let byte = self.input.byte(self.offset);
            let digit: u64 = if byte >= 48 && byte <= 57 {
                u64 { bits= byte.bits - 48 }
            } else if byte >= 65 && byte <= 70 {
                u64 { bits= byte.bits - 55 }
            } else if byte >= 97 && byte <= 102 {
                u64 { bits= byte.bits - 87 }
            } else {
                break;
            };
            if size > 1152921504606846975 {
                return self.failure(InvalidChunkSize, self.offset, false);
            }
            size = size * 16 + digit;
            self.offset = self.offset + 1;
        }
        if self.offset == 0 {
            return self.failure(InvalidChunkSize, 0, self.input.length() == 0);
        }
        while self.offset < self.input.length() limit Iterations(18446744073709551615) {
            self.whitespace();
            if self.offset == self.input.length() {
                return self.failure(InvalidChunkExtension, self.offset, true);
            }
            if self.input.byte(self.offset) != 59 {
                return self.failure(InvalidChunkExtension, self.offset, false);
            }
            self.offset = self.offset + 1;
            self.whitespace();
            if !self.token() {
                return self.failure(InvalidChunkExtension, self.offset, self.offset == self.input.length());
            }
            let before_whitespace = self.offset;
            self.whitespace();
            if self.offset < self.input.length() && self.input.byte(self.offset) == 61 {
                self.offset = self.offset + 1;
                self.whitespace();
                if self.offset < self.input.length() && self.input.byte(self.offset) == 34 {
                    self.offset = self.offset + 1;
                    while true limit Iterations(18446744073709551615) {
                        if self.offset == self.input.length() {
                            return self.failure(InvalidChunkExtension, self.offset, true);
                        }
                        let byte = self.input.byte(self.offset);
                        self.offset = self.offset + 1;
                        if byte == 34 { break; }
                        if byte == 92 {
                            if self.offset == self.input.length() {
                                return self.failure(InvalidChunkExtension, self.offset, true);
                            }
                            let escaped = self.input.byte(self.offset);
                            if escaped != 9 && (escaped < 32 || escaped == 127) {
                                return self.failure(InvalidChunkExtension, self.offset, false);
                            }
                            self.offset = self.offset + 1;
                        } else if byte != 9 && (byte < 32 || byte == 127) {
                            return self.failure(InvalidChunkExtension, self.offset - 1, false);
                        }
                    }
                } else if !self.token() {
                    return self.failure(InvalidChunkExtension, self.offset, self.offset == self.input.length());
                }
            } else {
                self.offset = before_whitespace;
            }
        }
        if self.complete { return Complete(size, 0); }
        return NeedMore;
    }
}

public flow decode_chunk_size_line_prefix(input: bytes, limit: usize) -> HttpDecodeStep<u64> ![] {
    let end = if input.length < limit { input.length } else { limit };
    let reader = WireReader { input= input, end= end };
    let (line, consumed, complete) = match reader.line(0) {
        Complete(line, consumed) => (line, consumed, true),
        Malformed(failure) => { return Malformed(failure); },
        NeedMore => (WireView { input= input, begin= 0, end= end }.partial(), 0, false),
    };
    var parser = ChunkLine { input= line, complete= complete, offset= 0 };
    return match parser.parse() {
        Complete(size, _) => Complete(size, consumed),
        Malformed(failure) => Malformed(failure),
        NeedMore => {
            if input.length >= limit {
                Malformed(HttpDecodeFailure { kind= LimitExceeded, offset= limit })
            } else {
                NeedMore
            }
        },
    };
}
