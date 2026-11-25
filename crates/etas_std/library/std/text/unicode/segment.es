module std.text.unicode;

enum Gcb { Other, CR, LF, Control, L, V, T, LV, LVT, Extend, ZWJ, SpacingMark, Prepend, RegionalIndicator, }
enum Incb { None, Consonant, Extend, Linker, }
type GraphemeRange = { start: u32, end: u32, kind: Gcb };
type IndicRange = { start: u32, end: u32, kind: Incb };
type GraphemeProperties = { kind: Gcb, indic: Incb, pictographic: bool };
type GraphemeData = {};

let GRAPHEME_DATA = GraphemeData {};

impl GraphemeData {
    flow grapheme(self, code: u32) -> Gcb ![] {
        if code >= 44032 && code < 55204 {
            return if (code - 44032) % 28 == 0 { Gcb.LV } else { Gcb.LVT };
        }
        var low: usize = 0;
        var high = GRAPHEME_RANGES.length;
        while low < high {
            let middle = low + (high - low) / 2;
            let range = GRAPHEME_RANGES.get(middle).unwrap();
            if code < range.start { high = middle; }
            else if code > range.end { low = middle + 1; }
            else { return range.kind; }
        }
        return Gcb.Other;
    }

    flow indic(self, code: u32) -> Incb ![] {
        var low: usize = 0;
        var high = INDIC_RANGES.length;
        while low < high {
            let middle = low + (high - low) / 2;
            let range = INDIC_RANGES.get(middle).unwrap();
            if code < range.start { high = middle; }
            else if code > range.end { low = middle + 1; }
            else { return range.kind; }
        }
        return Incb.None;
    }

    flow pictographic(self, code: u32) -> bool ![] {
        var low: usize = 0;
        var high = PICTOGRAPHIC_RANGES.length;
        while low < high {
            let middle = low + (high - low) / 2;
            let (start, end) = PICTOGRAPHIC_RANGES.get(middle).unwrap();
            if code < start { high = middle; }
            else if code > end { low = middle + 1; }
            else { return true; }
        }
        return false;
    }

    flow properties(self, scalar: char) -> GraphemeProperties ![] {
        let code = scalar.to_u32();
        return GraphemeProperties {
            kind= self.grapheme(code), indic= self.indic(code), pictographic= self.pictographic(code),
        };
    }
}

type GraphemeContext = {
    private previous: Option<Gcb>,
    private regional_count: usize,
    private pictographic_extend: bool,
    private zwj_after_pictographic: bool,
    private conjunct: bool,
    private linker_seen: bool,
};

impl GraphemeContext {
    flow new() -> GraphemeContext ![] {
        return GraphemeContext {
            previous= None, regional_count= 0, pictographic_extend= false,
            zwj_after_pictographic= false, conjunct= false, linker_seen= false,
        };
    }

    flow breaks(self, properties: GraphemeProperties) -> bool ![] {
        let previous = match self.previous { Some(kind) => kind, None => { return false; } };
        let current = properties.kind;
        if previous == Gcb.CR && current == Gcb.LF { return false; }
        if previous == Gcb.Control || previous == Gcb.CR || previous == Gcb.LF
            || current == Gcb.Control || current == Gcb.CR || current == Gcb.LF {
            return true;
        }
        if previous == Gcb.L && (current == Gcb.L || current == Gcb.V || current == Gcb.LV || current == Gcb.LVT) {
            return false;
        }
        if (previous == Gcb.LV || previous == Gcb.V) && (current == Gcb.V || current == Gcb.T) {
            return false;
        }
        if (previous == Gcb.LVT || previous == Gcb.T) && current == Gcb.T { return false; }
        if current == Gcb.Extend || current == Gcb.ZWJ || current == Gcb.SpacingMark || previous == Gcb.Prepend {
            return false;
        }
        if self.conjunct && self.linker_seen && properties.indic == Incb.Consonant { return false; }
        if self.zwj_after_pictographic && properties.pictographic { return false; }
        if previous == Gcb.RegionalIndicator && current == Gcb.RegionalIndicator && self.regional_count % 2 == 1 {
            return false;
        }
        return true;
    }

    flow advance(var self, properties: GraphemeProperties) -> unit ![] {
        let current = properties.kind;
        self.regional_count = if current == Gcb.RegionalIndicator { self.regional_count + 1 } else { 0 };
        self.zwj_after_pictographic = current == Gcb.ZWJ && self.pictographic_extend;
        self.pictographic_extend = if properties.pictographic {
            true
        } else if current == Gcb.Extend {
            self.pictographic_extend
        } else {
            false
        };
        if properties.indic == Incb.Consonant {
            self.conjunct = true;
            self.linker_seen = false;
        } else if properties.indic == Incb.Extend && self.conjunct {
        } else if properties.indic == Incb.Linker && self.conjunct {
            self.linker_seen = true;
        } else {
            self.conjunct = false;
            self.linker_seen = false;
        }
        self.previous = Some(current);
    }
}

impl char {
    flow grapheme_utf8_width(self) -> usize ![] {
        if self.bits < 128 { return 1; }
        if self.bits < 2048 { return 2; }
        if self.bits < 65536 { return 3; }
        return 4;
    }
}

type ClusterCursor = {
    private remaining: Iter<char>,
    private offset: usize,
    private start: usize,
    private context: GraphemeContext,
};

impl ClusterCursor {
    flow ranges(self) -> Iter<(usize, usize)> ![] {
        return Iter { next= () => {
            var remaining = self.remaining;
            var offset = self.offset;
            var context = self.context;
            while true {
                match (remaining.next)() {
                    None => {
                        if offset == self.start { return None; }
                        return Some(((self.start, offset), Iter.empty()));
                    },
                    Some((scalar, tail)) => {
                        let properties = GRAPHEME_DATA.properties(scalar);
                        let boundary = context.breaks(properties);
                        let at = offset;
                        context.advance(properties);
                        offset = offset + scalar.grapheme_utf8_width();
                        if boundary {
                            return Some(((self.start, at), ClusterCursor {
                                remaining= tail, offset= offset, start= at, context= context,
                            }.ranges()));
                        }
                        remaining = tail;
                    },
                };
            }
            return std.core.abort("grapheme iterator limit exceeded");
        } };
    }
}

public flow grapheme_indices(value: string) -> Iter<(usize, string)> ![] {
    let cursor = ClusterCursor { remaining= std.text.chars(value), offset= 0, start= 0, context= GraphemeContext.new() };
    return cursor.ranges().map((range: (usize, usize)) => {
        let (start, end) = range;
        let part = std.text.get_byte_slice(value, [start, end)).unwrap();
        return (start, part);
    });
}

public flow graphemes(value: string) -> Iter<string> ![] {
    let cursor = ClusterCursor { remaining= std.text.chars(value), offset= 0, start= 0, context= GraphemeContext.new() };
    return cursor.ranges().map((range: (usize, usize)) => {
        let (start, end) = range;
        return std.text.get_byte_slice(value, [start, end)).unwrap();
    });
}

public flow grapheme_count(value: string) -> usize ![] {
    let cursor = ClusterCursor { remaining= std.text.chars(value), offset= 0, start= 0, context= GraphemeContext.new() };
    var count: usize = 0;
    for range in cursor.ranges() {
        count = count + 1;
    }
    return count;
}

public flow reverse_graphemes(value: string) -> string ![] {
    let cursor = ClusterCursor { remaining= std.text.chars(value), offset= 0, start= 0, context= GraphemeContext.new() };
    var parts: List<string> = [];
    for (start, end) in cursor.ranges() {
        parts = std.text.get_byte_slice(value, [start, end)).unwrap() :: parts;
    }
    var output = "";
    for part in parts {
        output = output + part;
    }
    return output;
}

public flow grapheme_slice<I ~ std.core.Index>(value: string, range: Range<I>) -> string ![Error<std.text.TextError>] {
    var start = match range.start::std.core.Index.to_usize() {
        Some(index) => index,
        None => perform Error.raise<std.text.TextError>(std.text.InvalidRange),
    };
    var end = match range.end::std.core.Index.to_usize() {
        Some(index) => index,
        None => perform Error.raise<std.text.TextError>(std.text.InvalidRange),
    };
    let maximum = usize.maximum();
    if !range.include_start {
        if start == maximum { perform Error.raise<std.text.TextError>(std.text.InvalidRange); }
        start = start + 1;
    }
    if range.include_end {
        if end == maximum { perform Error.raise<std.text.TextError>(std.text.InvalidRange); }
        end = end + 1;
    }
    if start > end { perform Error.raise<std.text.TextError>(std.text.InvalidRange); }
    if end == 0 { return ""; }
    let cursor = ClusterCursor { remaining= std.text.chars(value), offset= 0, start= 0, context= GraphemeContext.new() };
    var ordinal: usize = 0;
    var output = "";
    for (begin, finish) in cursor.ranges() {
        if ordinal >= start {
            output = output + std.text.get_byte_slice(value, [begin, finish)).unwrap();
        }
        ordinal = ordinal + 1;
        if ordinal == end { return output; }
    }
    perform Error.raise<std.text.TextError>(std.text.InvalidRange);
}
