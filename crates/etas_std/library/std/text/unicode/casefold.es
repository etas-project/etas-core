module std.text.unicode;

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

public flow casefold(value: string) -> string ![] {
    var output: Array<string> = [];
    for scalar in std.text.chars(value) limit Iterations(18446744073709551615) {
        let mapped = match CASEFOLD_MAP.lookup(scalar) {
            Some(text) => text,
            None => std.text.from_char(scalar),
        };
        output = output.push(mapped);
    }
    return std.text.concat(output);
}
