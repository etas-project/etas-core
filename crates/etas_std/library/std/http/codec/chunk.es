module std.http.codec;

import std.http.codec.HttpCodecFailureKind.{InvalidChunkExtension, InvalidChunkSize, LimitExceeded};
import std.http.codec.HttpDecodeStep.{Complete, Malformed, NeedMore};

enum ChunkGrammar { Digits, AfterSize, BeforeName, Name, AfterName, BeforeValue, TokenValue, Quoted, Escape, AfterValue }

type ChunkLine = { private offset: usize, private size: u64, private state: ChunkGrammar };

impl ChunkLine {
    flow new() -> ChunkLine ![] {
        return ChunkLine { offset= 0, size= 0, state= ChunkGrammar.Digits };
    }

    // The offset is relative to the size line, including for a fragmented extension.
    flow parse(var self, line: WireView, complete: bool) -> HttpDecodeStep<u64> ![] {
        while self.offset < line.length() {
            let byte = line.byte(self.offset);
            let space = byte == 32 || byte == 9;
            match self.state {
                ChunkGrammar.Digits => {
                    let digit: u64 = if byte >= 48 && byte <= 57 { u64 { bits= byte.bits - 48 } }
                        else if byte >= 65 && byte <= 70 { u64 { bits= byte.bits - 55 } }
                        else if byte >= 97 && byte <= 102 { u64 { bits= byte.bits - 87 } }
                        else {
                            if self.offset == 0 {
                                return Malformed(HttpDecodeFailure { kind= InvalidChunkSize, offset= 0 });
                            }
                            self.state = ChunkGrammar.AfterSize;
                            continue;
                        };
                    if self.size > 1152921504606846975 {
                        return Malformed(HttpDecodeFailure { kind= InvalidChunkSize, offset= self.offset });
                    }
                    self.size = self.size * 16 + digit;
                },
                ChunkGrammar.BeforeName => {
                    if !space {
                        if !byte.is_http_token() { break; }
                        self.state = ChunkGrammar.Name;
                    }
                },
                ChunkGrammar.Name => {
                    if !byte.is_http_token() { self.state = ChunkGrammar.AfterName; continue; }
                },
                ChunkGrammar.AfterName => {
                    if byte == 61 { self.state = ChunkGrammar.BeforeValue; }
                    else if byte == 59 { self.state = ChunkGrammar.BeforeName; }
                    else if !space { break; }
                },
                ChunkGrammar.BeforeValue => {
                    if byte == 34 { self.state = ChunkGrammar.Quoted; }
                    else if byte.is_http_token() { self.state = ChunkGrammar.TokenValue; }
                    else if !space { break; }
                },
                ChunkGrammar.TokenValue => {
                    if !byte.is_http_token() { self.state = ChunkGrammar.AfterSize; continue; }
                },
                ChunkGrammar.Quoted => {
                    if byte == 34 { self.state = ChunkGrammar.AfterValue; }
                    else if byte == 92 { self.state = ChunkGrammar.Escape; }
                    else if byte != 9 && (byte < 32 || byte == 127) { break; }
                },
                ChunkGrammar.Escape => {
                    if byte != 9 && (byte < 32 || byte == 127) { break; }
                    self.state = ChunkGrammar.Quoted;
                },
                ChunkGrammar.AfterValue => { self.state = ChunkGrammar.AfterSize; continue; },
                ChunkGrammar.AfterSize => {
                    if byte == 59 { self.state = ChunkGrammar.BeforeName; }
                    else if !space { break; }
                },
            };
            self.offset = self.offset + 1;
        }
        if self.offset < line.length() {
            return Malformed(HttpDecodeFailure { kind= InvalidChunkExtension, offset= self.offset });
        }
        if !complete { return NeedMore; }
        if self.offset == 0 { return Malformed(HttpDecodeFailure { kind= InvalidChunkSize, offset= 0 }); }
        return match self.state {
            ChunkGrammar.Digits => Complete(self.size, 0),
            ChunkGrammar.Name => Complete(self.size, 0),
            ChunkGrammar.TokenValue => Complete(self.size, 0),
            ChunkGrammar.AfterValue => Complete(self.size, 0),
            _ => Malformed(HttpDecodeFailure { kind= InvalidChunkExtension, offset= self.offset }),
        };
    }
}

public flow decode_chunk_size_line_prefix(input: bytes, limit: usize) -> HttpDecodeStep<u64> ![] {
    let end = if input.length < limit { input.length } else { limit };
    let reader = WireReader { input= input, end= end };
    let (line, consumed, complete) = match reader.line(0, 0) {
        Complete(line, consumed) => (line, consumed, true),
        Malformed(failure) => { return Malformed(failure); },
        NeedMore => (WireView { input= input, begin= 0, end= end }.partial(), 0, false),
    };
    var parser = ChunkLine.new();
    return match parser.parse(line, complete) {
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
