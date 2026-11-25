module std.text.unicode;

type CombiningEntry = { scalar: u32, class: usize };
type Decomposition = { scalar: u32, compatibility: bool, parts: Array<u32> };
type Composition = { key: u128, scalar: u32 };
type NormalizationData = {};

let NORMALIZER = NormalizationData {};

impl NormalizationData {
    flow combining_class(self, scalar: u32) -> usize ![] {
        var low: usize = 0;
        var high = COMBINING_CLASSES.length;
        while low < high {
            let middle = low + (high - low) / 2;
            let record = COMBINING_CLASSES.get(middle).unwrap();
            if scalar < record.scalar { high = middle; }
            else if scalar > record.scalar { low = middle + 1; }
            else { return record.class; }
        }
        return 0;
    }

    flow decomposition(self, scalar: u32, compatibility: bool) -> Option<Array<u32>> ![] {
        var low: usize = 0;
        var high = DECOMPOSITIONS.length;
        while low < high {
            let middle = low + (high - low) / 2;
            let record = DECOMPOSITIONS.get(middle).unwrap();
            if scalar < record.scalar { high = middle; }
            else if scalar > record.scalar { low = middle + 1; }
            else {
                if record.compatibility && !compatibility { return None; }
                return Some(record.parts);
            }
        }
        return None;
    }

    flow composition(self, starter: u32, scalar: u32) -> Option<u32> ![] {
        if starter >= 4352 && starter < 4371 && scalar >= 4449 && scalar < 4470 {
            return Some(44032 + ((starter - 4352) * 21 + scalar - 4449) * 28);
        }
        if starter >= 44032 && starter < 55204 && (starter - 44032) % 28 == 0
            && scalar > 4519 && scalar < 4547 {
            return Some(starter + scalar - 4519);
        }
        let key = starter.bits * 2097152 + scalar.bits;
        var low: usize = 0;
        var high = COMPOSITIONS.length;
        while low < high {
            let middle = low + (high - low) / 2;
            let record = COMPOSITIONS.get(middle).unwrap();
            if key < record.key { high = middle; }
            else if key > record.key { low = middle + 1; }
            else { return Some(record.scalar); }
        }
        return None;
    }
}

impl Normalization {
    flow compatibility(self) -> bool ![] {
        return match self { Normalization.NFKC => true, Normalization.NFKD => true, _ => false };
    }

    flow composed(self) -> bool ![] {
        return match self { Normalization.NFC => true, Normalization.NFKC => true, _ => false };
    }
}

type CanonicalOrder = {
    private output: Array<u32>,
    private buckets: Array<List<u32>>,
    private marked: usize,
};

impl CanonicalOrder {
    flow new() -> CanonicalOrder ![] {
        var buckets: Array<List<u32>> = [];
        var class: usize = 0;
        while class < 256 {
            buckets = buckets.push([]);
            class = class + 1;
        }
        return CanonicalOrder { output= [], buckets= buckets, marked= 0 };
    }

    flow flush(var self) -> unit ![] {
        if self.marked == 0 { return; }
        var class: usize = 1;
        while class < 256 {
            let values = self.buckets.get(class).unwrap();
            if !values.is_empty() {
                for scalar in values.reverse() {
                    self.output = self.output.push(scalar);
                }
                self.buckets = self.buckets.set(class, [])?.unwrap();
            }
            class = class + 1;
        }
        self.marked = 0;
    }

    flow push(var self, scalar: u32) -> unit ![] {
        let class = NORMALIZER.combining_class(scalar);
        if class == 0 {
            self.flush();
            self.output = self.output.push(scalar);
        } else {
            let bucket = self.buckets.get(class).unwrap();
            self.buckets = self.buckets.set(class, scalar :: bucket)?.unwrap();
            self.marked = self.marked + 1;
        }
    }

    flow decompose(var self, scalar: u32, compatibility: bool) -> unit ![] {
        var stack: List<u32> = scalar :: [];
        while !stack.is_empty() {
            let (head, tail) = match stack {
                value :: rest => (value, rest),
                [] => std.core.abort("nonempty decomposition stack invariant"),
            };
            stack = tail;
            if head >= 44032 && head < 55204 {
                let index = head - 44032;
                let trailing = index % 28;
                if trailing != 0 { stack = (4519 + trailing) :: stack; }
                stack = (4449 + (index % 588) / 28) :: stack;
                stack = (4352 + index / 588) :: stack;
            } else {
                match NORMALIZER.decomposition(head, compatibility) {
                    Some(parts) => {
                        var index = parts.length;
                        while index > 0 {
                            index = index - 1;
                            stack = parts.get(index).unwrap() :: stack;
                        }
                    },
                    None => { self.push(head); },
                };
            }
        }
    }
}

public flow normalize(value: string, form: Normalization) -> string ![] {
    var order = CanonicalOrder.new();
    let compatibility = form.compatibility();
    for scalar in std.text.chars(value) {
        order.decompose(scalar.to_u32(), compatibility);
    }
    order.flush();
    var output = order.output;
    if form.composed() {
        var composed: Array<u32> = [];
        var starter: Option<usize> = None;
        var last_class: usize = 0;
        for scalar in output {
            let class = NORMALIZER.combining_class(scalar);
            match starter {
                Some(index) => {
                    if last_class == 0 || last_class < class {
                        match NORMALIZER.composition(composed.get(index).unwrap(), scalar) {
                            Some(value) => {
                                composed = composed.set(index, value)?.unwrap();
                                continue;
                            },
                            None => {},
                        };
                    }
                },
                None => {},
            };
            composed = composed.push(scalar);
            if class == 0 { starter = Some(composed.length - 1); }
            last_class = class;
        }
        output = composed;
    }
    var parts: Array<string> = [];
    for scalar in output {
        parts = parts.push(std.text.from_char(char { bits= scalar.bits }));
    }
    return std.text.concat(parts);
}

public flow is_normalized(value: string, form: Normalization) -> bool ![] {
    let normalized = normalize(value, form);
    return normalized == value;
}
