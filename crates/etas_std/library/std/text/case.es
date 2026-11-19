module std.text;

type CaseMapping = { code: u32, text: string };
type CaseMap = { private entries: Array<CaseMapping> };

impl CaseMap {
    flow lookup(self, scalar: char) -> Option<string> ![] {
        let code = scalar.to_u32();
        var low: usize = 0;
        var high = self.entries.length;
        while low < high limit Iterations(32) {
            let middle = low + (high - low) / 2;
            let entry = self.entries.get(middle).unwrap();
            if code < entry.code {
                high = middle;
            } else if code > entry.code {
                low = middle + 1;
            } else {
                return Some(entry.text);
            }
        }
        return None;
    }
}

type CaseSet = { private ranges: Array<(u32, u32)> };

impl CaseSet {
    flow contains(self, scalar: char) -> bool ![] {
        let code = scalar.to_u32();
        var low: usize = 0;
        var high = self.ranges.length;
        while low < high limit Iterations(32) {
            let middle = low + (high - low) / 2;
            let (start, end) = self.ranges.get(middle).unwrap();
            if code < start {
                high = middle;
            } else if code > end {
                low = middle + 1;
            } else {
                return true;
            }
        }
        return false;
    }
}

public flow lowercase(value: string) -> string ![] {
    var changed = false;
    for scalar in chars(value) limit Iterations(18446744073709551615) {
        if LOWERCASE_MAP.lookup(scalar).is_some() { changed = true; break; }
    }
    if !changed { return value; }
    var output: Array<string> = [];
    var preceded_by_cased = false;
    var pending_sigma: Option<usize> = None;
    for scalar in chars(value) limit Iterations(18446744073709551615) {
        if !CASE_IGNORABLE_SET.contains(scalar) {
            let following_is_cased = CASED_SET.contains(scalar);
            match pending_sigma {
                Some(index) => {
                    if !following_is_cased {
                        output = output.set(index, "ς")?.unwrap();
                    }
                },
                None => {},
            };
            pending_sigma = None;
            if scalar == 'Σ' && preceded_by_cased { pending_sigma = Some(output.length); }
            preceded_by_cased = following_is_cased;
        }
        let mapped = match LOWERCASE_MAP.lookup(scalar) {
            Some(text) => text,
            None => from_char(scalar),
        };
        output = output.push(mapped);
    }
    match pending_sigma {
        Some(index) => { output = output.set(index, "ς")?.unwrap(); },
        None => {},
    };
    return concat(output);
}

public flow uppercase(value: string) -> string ![] {
    var changed = false;
    for scalar in chars(value) limit Iterations(18446744073709551615) {
        if UPPERCASE_MAP.lookup(scalar).is_some() { changed = true; break; }
    }
    if !changed { return value; }
    var output: Array<string> = [];
    for scalar in chars(value) limit Iterations(18446744073709551615) {
        let mapped = match UPPERCASE_MAP.lookup(scalar) {
            Some(text) => text,
            None => from_char(scalar),
        };
        output = output.push(mapped);
    }
    return concat(output);
}
