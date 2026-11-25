module std.text;

type IndexedScalars = { private remaining: Iter<char>, private offset: usize };

impl IndexedScalars {
    flow iter(self) -> Iter<(usize, char)> ![] {
        return Iter { next= () => match (self.remaining.next)() {
            None => None,
            Some((scalar, remaining)) => Some((
                (self.offset, scalar),
                IndexedScalars { remaining= remaining, offset= self.offset + scalar_width(scalar) }.iter(),
            )),
        } };
    }
}

public flow char_indices(text: string) -> Iter<(usize, char)> ![] {
    let input = chars(text);
    return Iter { next= () => match (input.next)() {
        None => None,
        Some((scalar, remaining)) => Some((
            (0, scalar), IndexedScalars { remaining= remaining, offset= scalar_width(scalar) }.iter(),
        )),
    } };
}

type LineCursor = { private source: string, private start: usize };

impl LineCursor {
    flow iter(self) -> Iter<string> ![] {
        return Iter { next= () => {
            let input = self.source.utf8;
            var offset = self.start;
            while offset < input.length {
                if input.get(offset) == Some(10) {
                    let end = if offset > self.start && input.get(offset - 1) == Some(13) {
                        offset - 1
                    } else {
                        offset
                    };
                    let part = TextSpan { source= self.source, start= self.start, end= end }.materialize();
                    return Some((part, LineCursor { source= self.source, start= offset + 1 }.iter()));
                }
                offset = offset + 1;
            }
            if self.start == input.length { return None; }
            let part = TextSpan { source= self.source, start= self.start, end= input.length }.materialize();
            return Some((part, Iter.empty()));
        } };
    }
}

public flow lines(text: string) -> Iter<string> ![] {
    let input = text.utf8;
    return Iter { next= () => {
        var offset: usize = 0;
        while offset < input.length {
            if input.get(offset) == Some(10) {
                let end = if offset > 0 && input.get(offset - 1) == Some(13) { offset - 1 } else { offset };
                let part = TextSpan { source= text, start= 0, end= end }.materialize();
                return Some((part, LineCursor { source= text, start= offset + 1 }.iter()));
            }
            offset = offset + 1;
        }
        if input.length == 0 { return None; }
        return Some((text, Iter.empty()));
    } };
}

type MatchCursor = { private matcher: BytePattern, private input: bytes, private start: usize };

impl MatchCursor {
    flow iter(self) -> Iter<(usize, usize)> ![] {
        return Iter { next= () => match self.matcher.find(self.input, self.start) {
            None => None,
            Some((start, end)) => Some(((start, end), MatchCursor {
                matcher= self.matcher, input= self.input, start= end,
            }.iter())),
        } };
    }
}

type BoundaryMatches = { private remaining: Iter<char>, private offset: usize };

impl BoundaryMatches {
    flow iter(self) -> Iter<(usize, usize)> ![] {
        return Iter { next= () => match (self.remaining.next)() {
            None => None,
            Some((scalar, remaining)) => {
                let offset = self.offset + scalar_width(scalar);
                Some(((offset, offset), BoundaryMatches { remaining= remaining, offset= offset }.iter()))
            },
        } };
    }
}

public flow find_all(text: string, pattern: string) -> Iter<(usize, usize)> ![] {
    if pattern == "" {
        let remaining = chars(text);
        return Iter { next= () => Some(((0, 0), BoundaryMatches { remaining= remaining, offset= 0 }.iter())) };
    }
    let matcher = BytePattern.new(pattern);
    let input = text.utf8;
    return Iter { next= () => match matcher.find(input, 0) {
        None => None,
        Some((start, end)) => Some(((start, end), MatchCursor { matcher= matcher, input= input, start= end }.iter())),
    } };
}

type EmptySegments = { private remaining: Iter<char> };

impl EmptySegments {
    flow iter(self) -> Iter<string> ![] {
        return Iter { next= () => match (self.remaining.next)() {
            Some((scalar, remaining)) => Some((from_char(scalar), EmptySegments { remaining= remaining }.iter())),
            None => Some(("", Iter.empty())),
        } };
    }
}

type SplitCursor = { private source: string, private matcher: BytePattern, private start: usize };

impl SplitCursor {
    flow iter(self) -> Iter<string> ![] {
        return Iter { next= () => match self.matcher.find(self.source.utf8, self.start) {
            Some((start, end)) => {
                let part = TextSpan { source= self.source, start= self.start, end= start }.materialize();
                Some((part, SplitCursor { source= self.source, matcher= self.matcher, start= end }.iter()))
            },
            None => {
                let part = TextSpan { source= self.source, start= self.start, end= self.source.length }.materialize();
                Some((part, Iter.empty()))
            },
        } };
    }
}

public flow split_iter(text: string, separator: string) -> Iter<string> ![] {
    if separator == "" {
        let remaining = chars(text);
        return Iter { next= () => Some(("", EmptySegments { remaining= remaining }.iter())) };
    }
    let matcher = BytePattern.new(separator);
    return Iter { next= () => match matcher.find(text.utf8, 0) {
        Some((start, end)) => {
            let part = TextSpan { source= text, start= 0, end= start }.materialize();
            Some((part, SplitCursor { source= text, matcher= matcher, start= end }.iter()))
        },
        None => Some((text, Iter.empty())),
    } };
}

type WhitespaceCursor = { private source: string, private remaining: Iter<char>, private offset: usize };

impl WhitespaceCursor {
    flow iter(self) -> Iter<string> ![] {
        return Iter { next= () => {
            var remaining = self.remaining;
            var offset: usize = self.offset;
            var start: Option<usize> = None;
            while true {
                match (remaining.next)() {
                    None => {
                        return match start {
                            Some(begin) => Some((TextSpan { source= self.source, start= begin, end= offset }.materialize(), Iter.empty())),
                            None => None,
                        };
                    },
                    Some((scalar, tail)) => {
                        let next = offset + scalar_width(scalar);
                        if scalar.is_whitespace() {
                            match start {
                                Some(begin) => {
                                    let part = TextSpan { source= self.source, start= begin, end= offset }.materialize();
                                    return Some((part, WhitespaceCursor { source= self.source, remaining= tail, offset= next }.iter()));
                                },
                                None => {},
                            };
                        } else if start.is_none() {
                            start = Some(offset);
                        }
                        offset = next;
                        remaining = tail;
                    },
                };
            }
            return std.core.abort("whitespace iterator limit exceeded");
        } };
    }
}

public flow split_whitespace(text: string) -> Iter<string> ![] {
    let input = chars(text);
    return Iter { next= () => {
        var remaining = input;
        var offset: usize = 0;
        var start: Option<usize> = None;
        while true {
            match (remaining.next)() {
                None => {
                    return match start {
                        Some(begin) => Some((TextSpan { source= text, start= begin, end= offset }.materialize(), Iter.empty())),
                        None => None,
                    };
                },
                Some((scalar, tail)) => {
                    let next = offset + scalar_width(scalar);
                    if scalar.is_whitespace() {
                        match start {
                            Some(begin) => {
                                let part = TextSpan { source= text, start= begin, end= offset }.materialize();
                                return Some((part, WhitespaceCursor { source= text, remaining= tail, offset= next }.iter()));
                            },
                            None => {},
                        };
                    } else if start.is_none() {
                        start = Some(offset);
                    }
                    offset = next;
                    remaining = tail;
                },
            };
        }
        return std.core.abort("whitespace iterator limit exceeded");
    } };
}
