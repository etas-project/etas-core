module std.http.codec;

import std.http.codec.HttpCodecFailureKind.{InvalidHeader, InvalidLineEnding, InvalidStatusCode, InvalidStatusLine, UnsupportedHttpVersion};
import std.http.codec.HttpDecodeStep.{Complete, Malformed, NeedMore};

type WireView = { private input: bytes, private begin: usize, private end: usize };

impl WireView {
    flow length(self) -> usize ![] {
        return self.end - self.begin;
    }

    flow byte(self, index: usize) -> u8 ![] {
        return self.input.get(self.begin + index).unwrap();
    }

    flow span(self, begin: usize, end: usize) -> WireView ![] {
        return WireView { input= self.input, begin= self.begin + begin, end= self.begin + end };
    }

    flow to_bytes(self) -> bytes ![] {
        return self.input[self.begin, self.end)?.unwrap();
    }

    flow text(self) -> Result<string, std.codec.text.TextCodecError> ![] {
        return std.codec.text.utf8_decode(self.to_bytes(), std.codec.text.Strict);
    }

    flow invalid_utf8(self) -> Option<usize> ![] {
        var index: usize = 0;
        while index < self.length() {
            let start = index;
            let lead = self.byte(index);
            let width: usize = if lead < 128 { 1 }
                else if lead >= 194 && lead <= 223 { 2 }
                else if lead >= 224 && lead <= 239 { 3 }
                else if lead >= 240 && lead <= 244 { 4 }
                else { return Some(start); };
            var part: usize = 1;
            while part < width {
                if part >= self.length() - start { return Some(start); }
                let byte = self.byte(start + part);
                if byte < 128 || byte > 191 { return Some(start); }
                if part == 1 && (
                    (lead == 224 && byte < 160) || (lead == 237 && byte > 159)
                    || (lead == 240 && byte < 144) || (lead == 244 && byte > 143)
                ) {
                    return Some(start);
                }
                part = part + 1;
            }
            index = index + width;
        }
        return None;
    }

    flow partial(self) -> WireView ![] {
        let end = if self.end > self.begin && self.input.get(self.end - 1) == Some(13) {
            self.end - 1
        } else {
            self.end
        };
        return WireView { input= self.input, begin= self.begin, end= end };
    }

    flow status_prefix(self, complete: bool) -> Option<HttpDecodeFailure> ![] {
        let http10 = "HTTP/1.0 ".utf8;
        let http11 = "HTTP/1.1 ".utf8;
        let length = if self.length() < 9 { self.length() } else { 9 };
        var first = true;
        var second = true;
        var index: usize = 0;
        while index < length {
            first = first && self.byte(index) == http10.get(index).unwrap();
            second = second && self.byte(index) == http11.get(index).unwrap();
            index = index + 1;
        }
        if !first && !second {
            return Some(HttpDecodeFailure { kind= UnsupportedHttpVersion, offset= self.begin });
        }
        index = 9;
        while index < self.length() && index < 12 {
            let byte = self.byte(index);
            if byte < 48 || byte > 57 || (index == 9 && byte == 48) {
                return Some(HttpDecodeFailure { kind= InvalidStatusCode, offset= self.begin + index });
            }
            index = index + 1;
        }
        if (self.length() > 12 && self.byte(12) != 32) || (complete && self.length() < 13) {
            let bad = if self.length() < 12 { self.length() } else { 12 };
            return Some(HttpDecodeFailure { kind= InvalidStatusLine, offset= self.begin + bad });
        }
        index = 13;
        while index < self.length() {
            let byte = self.byte(index);
            if (byte < 32 || byte == 127) && byte != 9 {
                return Some(HttpDecodeFailure { kind= InvalidStatusLine, offset= self.begin + index });
            }
            index = index + 1;
        }
        return None;
    }

    flow status(self) -> Result<(string, i32, string), HttpDecodeFailure> ![] {
        var first_space: usize = 0;
        while first_space < self.length() && self.byte(first_space) != 32 {
            first_space = first_space + 1;
        }
        if first_space == self.length() {
            return Err(HttpDecodeFailure { kind= InvalidStatusLine, offset= self.begin });
        }
        let version = match self.span(0, first_space).text() {
            Ok(value) => value,
            Err(_) => { return Err(HttpDecodeFailure { kind= UnsupportedHttpVersion, offset= self.begin }); },
        };
        if version != "HTTP/1.0" && version != "HTTP/1.1" {
            return Err(HttpDecodeFailure { kind= UnsupportedHttpVersion, offset= self.begin });
        }
        let start = first_space + 1;
        if self.length() - start < 3 {
            return Err(HttpDecodeFailure { kind= InvalidStatusCode, offset= self.begin + start });
        }
        var status: i32 = 0;
        var index = start;
        while index < start + 3 {
            let byte = self.byte(index);
            if byte < 48 || byte > 57 {
                return Err(HttpDecodeFailure { kind= InvalidStatusCode, offset= self.begin + start });
            }
            status = status * 10 + i32 { bits= byte.bits - 48 };
            index = index + 1;
        }
        if status < 100 || (index < self.length() && self.byte(index) != 32) {
            return Err(HttpDecodeFailure { kind= InvalidStatusCode, offset= self.begin + start });
        }
        let reason_start = if index < self.length() { index + 1 } else { index };
        index = reason_start;
        while index < self.length() {
            let byte = self.byte(index);
            if (byte < 32 || byte == 127) && byte != 9 {
                return Err(HttpDecodeFailure { kind= InvalidStatusLine, offset= self.begin + reason_start });
            }
            index = index + 1;
        }
        let reason_view = self.span(reason_start, self.length());
        match reason_view.invalid_utf8() {
            Some(offset) => {
                return Err(HttpDecodeFailure { kind= InvalidStatusLine, offset= self.begin + reason_start + offset });
            },
            None => {},
        };
        let reason = reason_view.text().unwrap();
        return Ok((version, status, reason));
    }

    flow header_prefix(self) -> Option<usize> ![] {
        var value = false;
        var offset: usize = 0;
        while offset < self.length() {
            let byte = self.byte(offset);
            if !value && byte == 58 {
                if offset == 0 { return Some(offset); }
                value = true;
            } else if (!value && !byte.is_http_token())
                || (value && (byte < 32 || byte == 127) && byte != 9) {
                return Some(offset);
            }
            offset = offset + 1;
        }
        return None;
    }

    flow header(self) -> Result<HttpHeader, HttpCodecFailureKind> ![] {
        var colon: usize = 0;
        while colon < self.length() && self.byte(colon) != 58 {
            if !self.byte(colon).is_http_token() { return Err(InvalidHeader); }
            colon = colon + 1;
        }
        if colon == 0 || colon == self.length() { return Err(InvalidHeader); }
        var start = colon + 1;
        var end = self.length();
        while start < end && (self.byte(start) == 32 || self.byte(start) == 9) {
            start = start + 1;
        }
        while end > start && (self.byte(end - 1) == 32 || self.byte(end - 1) == 9) {
            end = end - 1;
        }
        var index = start;
        while index < end {
            let byte = self.byte(index);
            if (byte < 32 || byte == 127) && byte != 9 { return Err(InvalidHeader); }
            index = index + 1;
        }
        let name = match self.span(0, colon).text() {
            Ok(value) => value,
            Err(_) => { return Err(InvalidHeader); },
        };
        let value = match self.span(start, end).text() {
            Ok(value) => value,
            Err(_) => { return Err(InvalidHeader); },
        };
        return Ok(HttpHeader { name= name, value= value });
    }
}

type WireReader = { private input: bytes, private end: usize };

impl WireReader {
    flow resume_offset(self) -> usize ![] {
        if self.end > 0 && self.input.get(self.end - 1) == Some(13) { return self.end - 1; }
        return self.end;
    }

    flow line(self, start: usize, scanned: usize) -> HttpDecodeStep<WireView> ![] {
        var offset = scanned;
        while offset < self.end {
            let byte = self.input.get(offset).unwrap();
            if byte == 10 {
                return Malformed(HttpDecodeFailure { kind= InvalidLineEnding, offset= offset });
            }
            if byte == 13 {
                if self.end - offset < 2 { return NeedMore; }
                if self.input.get(offset + 1) != Some(10) {
                    return Malformed(HttpDecodeFailure { kind= InvalidLineEnding, offset= offset });
                }
                return Complete(WireView { input= self.input, begin= start, end= offset }, offset + 2);
            }
            offset = offset + 1;
        }
        return NeedMore;
    }

    flow head_end(self, scanned: usize) -> HttpDecodeStep<unit> ![] {
        var offset = scanned;
        while offset < self.end {
            let byte = self.input.get(offset).unwrap();
            if byte == 10 {
                return Malformed(HttpDecodeFailure { kind= InvalidLineEnding, offset= offset });
            }
            if byte == 13 {
                if self.end - offset < 2 { return NeedMore; }
                if self.input.get(offset + 1) != Some(10) {
                    return Malformed(HttpDecodeFailure { kind= InvalidLineEnding, offset= offset });
                }
                if offset >= 2 && self.input.get(offset - 2) == Some(13)
                    && self.input.get(offset - 1) == Some(10) {
                    return Complete((), offset + 2);
                }
                offset = offset + 2;
            } else {
                offset = offset + 1;
            }
        }
        return NeedMore;
    }
}

impl u8 {
    flow is_http_token(self) -> bool ![] {
        if self < 33 || self > 126 { return false; }
        return match self {
            40 => false,
            41 => false,
            60 => false,
            62 => false,
            64 => false,
            44 => false,
            59 => false,
            58 => false,
            92 => false,
            34 => false,
            47 => false,
            91 => false,
            93 => false,
            63 => false,
            61 => false,
            123 => false,
            125 => false,
            _ => true,
        };
    }
}
