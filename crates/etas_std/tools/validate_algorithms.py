#!/usr/bin/env python3
"""Exercise the maintained Etas algorithms through a compiled source SDK.

Usage: validate_algorithms.py --binary PATH --sysroot PATH
The sysroot must contain the freshly built std/manifest.json. No legacy
registry is involved. Float expectations come from Rust's standard library;
Unicode expectations come from the repository's existing UCD 17 fixtures.
"""
import argparse
import json
from pathlib import Path
import random
import re
import subprocess
import tempfile


def literal(value):
    text = json.dumps(value, ensure_ascii=False)
    return re.sub(r"\\u([0-9a-fA-F]{4})", r"\\u{\1}", text)


def array(values):
    return "[" + ", ".join(literal(value) if isinstance(value, str) else str(value) for value in values) + "]"


def run_suite(binary, sysroot, work, name, setup, checks):
    source = work / f"{name}.es"
    lines = ["module app.main;", "flow main(args: Array<string>) -> i32 {", setup]
    for index, (label, expression) in enumerate(checks, 1):
        lines.append(f"    if !({expression}) {{ return {index}; }} // {label}")
    lines.extend(["    return 0;", "}"])
    source.write_text("\n".join(lines) + "\n")
    command = [str(binary), "--no-config", "--std-provider", "artifact", "--sysroot", str(sysroot), "--format", "json", "run", str(source)]
    result = subprocess.run(command, capture_output=True, text=True, timeout=240)
    (work / f"{name}.stdout.json").write_text(result.stdout)
    (work / f"{name}.stderr.log").write_text(result.stderr)
    if result.returncode:
        raise RuntimeError(f"{name}: CLI exit {result.returncode}\n{result.stdout[:5000]}\n{result.stderr[:5000]}")
    report = json.loads(result.stdout)
    index = int(report["value"]["value"])
    if index:
        label, expression = checks[index - 1]
        raise AssertionError(f"{name}: {label}\n{expression}\n{source}")
    print(f"PASS {name}: {len(checks)} checks", flush=True)
    return len(checks)


def text_checks():
    checks = []
    def add(label, expression): checks.append((label, expression))
    for text, pattern in [("ababababac界", "ababac"), ("é界aé界", "é界"), ("aaaaaa", "aaa"), ("tool-output-42", "output")]:
        args = f"{literal(text)}, {literal(pattern)}"
        first = len(text[:text.index(pattern)].encode())
        last = len(text[:text.rindex(pattern)].encode())
        add("KMP contains", f"std.text.contains({args})")
        add("KMP find", f"std.text.find({args}) == Some({first})")
        add("KMP rfind including overlaps", f"std.text.rfind({args}) == Some({last})")
        add("nonoverlapping count", f"std.text.count({args}) == {text.count(pattern)}")
        add("replace", f"std.text.replace({args}, \"X\") == {literal(text.replace(pattern, 'X'))}")
        add("replace_n", f"std.text.replace_n({args}, \"X\", 1) == {literal(text.replace(pattern, 'X', 1))}")
        add("split", f"std.text.split({args}) == {array(text.split(pattern))}")
        add("split_iter", f"std.text.split_iter({args}).collect() == {array(text.split(pattern))}")
    add("find_from byte index", r'std.text.find_from("é界aé界", "é界", 2) == Some(6)')
    add("splitn", r'std.text.splitn("a界b界c", "界", 2) == ["a", "b界c"]')
    add("split_once", r'std.text.split_once("a界b界c", "界") == Some(("a", "b界c"))')
    add("rsplit_once", r'std.text.rsplit_once("a界b界c", "界") == Some(("a界b", "c"))')
    add("prefix", r'std.text.starts_with("é界tail", "é界")')
    add("suffix", r'std.text.ends_with("headé界", "é界")')
    add("strip prefix", r'std.text.strip_prefix("é界tail", "é界") == Some("tail")')
    add("strip suffix", r'std.text.strip_suffix("headé界", "é界") == Some("head")')
    add("trim", r'std.text.trim("  é界 \t") == "é界"')
    add("trim start", r'std.text.trim_start("  é界 \t") == "é界 \t"')
    add("trim end", r'std.text.trim_end("  é界 \t") == "  é界"')
    add("lines", r'std.text.lines("é\r\n界\nlog\rtext\n").collect() == ["é", "界", "log\rtext"]')
    add("whitespace", r'std.text.split_whitespace(" é 界\n log ").collect() == ["é", "界", "log"]')
    add("ASCII lowercase", r'std.text.ascii_lowercase("ÄABC界") == "Äabc界"')
    add("ASCII uppercase", r'std.text.ascii_uppercase("äabc界") == "äABC界"')
    add("ASCII comparison", r'std.text.eq_ignore_ascii_case("éAscii界", "éaSCII界")')
    add("byte slice", r'std.text.byte_slice("Aé界tail", [1, 6)) == "é界"')
    add("char slice", r'std.text.char_slice("Aé界tail", [1, 3)) == "é界"')
    add("replace range", r'std.text.replace_range("Aé界tail", [1, 6), "X") == "AXtail"')
    return "", checks


def collection_checks():
    setup = '''
    let original: Map<i32, i32> = Map.new().insert(1, 10).insert(2, 20);
    let updated = original.update(2, (old: Option<i32>) => Some(old.unwrap() + 7));
    let inserted = updated.update(3, (old: Option<i32>) => Some(old.unwrap_or(30)));
    let removed = inserted.update(1, (old: Option<i32>) => None);
    let merged = original.merge(Map.new().insert(2, 3).insert(4, 40), (key: i32, left: i32, right: i32) => key + left + right);
    let items: Array<i32> = [2, 1, 2, 3, 1, 2];
    let groups = std.collections.group_by(items.iter(), (value: i32) => value % 2);
    let counts = std.collections.count_by(items.iter(), (value: i32) => value % 2);
    let frequencies = std.collections.frequencies(items.iter());
    let combined = std.collections.merge_counts(frequencies, Map.new().insert(2, 5).insert(4, 1));
    let left = BitSet.from_indices(128, [3, 7, 65, 92, 126].iter());
    let right = BitSet.from_indices(128, [7, 9, 65, 100].iter());
    let queue: PriorityQueue<string, i32> = PriorityQueue.new().push("a", 3).push("b", 3).push("c", 1);
    let (pushed, first) = queue.push_pop("d", 2);
    let (replaced, second) = pushed.replace_top("e", 1);
    let (retained, immediate) = queue.push_pop("z", 9);
    let ranked: Array<(i32, i32)> = [(2, 0), (1, 1), (2, 2), (1, 3), (3, 4), (2, 5)];
'''
    checks = [
        ("Map update", "updated.get(2) == Some(27) && original.get(2) == Some(20)"),
        ("Map insert through update", "inserted.get(3) == Some(30) && inserted.len() == 3"),
        ("Map remove through update", "removed.get(1) == None && removed.len() == 2"),
        ("Map merge", "merged.get(2) == Some(25) && merged.get(4) == Some(40)"),
        ("group_by order", "groups.get(0) == Some([2, 2, 2]) && groups.get(1) == Some([1, 3, 1])"),
        ("count_by", "counts.get(0) == Some(3) && counts.get(1) == Some(3)"),
        ("frequencies", "frequencies.get(2) == Some(3) && frequencies.get(1) == Some(2)"),
        ("merge_counts", "combined.get(2) == Some(8) && combined.get(4) == Some(1)"),
        ("unique", "std.collections.unique(items) == [2, 1, 3]"),
        ("unique_by_key", "std.collections.unique_by_key(items, (value: i32) => value % 2) == [2, 1]"),
        ("BitSet count", "left.count() == 5"),
        ("BitSet union", "left.union(right).iter().collect() == [3, 7, 9, 65, 92, 100, 126]"),
        ("BitSet intersection", "left.intersection(right).iter().collect() == [7, 65]"),
        ("BitSet difference", "left.difference(right).iter().collect() == [3, 92, 126]"),
        ("BitSet symmetric difference", "left.symmetric_difference(right).iter().collect() == [3, 9, 92, 100, 126]"),
        ("BitSet subset", "left.intersection(right).is_subset(left)"),
        ("BitSet disjoint", "left.difference(right).is_disjoint(right)"),
        ("heap push_pop FIFO", 'first == "a"'),
        ("heap replace_top", 'second == Some("b") && replaced.to_sorted_array() == ["d", "c", "e"]'),
        ("heap immediate push_pop", 'immediate == "z" && retained.to_sorted_array() == ["a", "b", "c"]'),
        ("stable key sorting", "std.collections.stable_sort_by_key(ranked, (item: (i32, i32)) => { let (key, _) = item; return key; }) == [(1, 1), (1, 3), (2, 0), (2, 2), (2, 5), (3, 4)]"),
        ("sort_by_key", "std.collections.sort_by_key(items, (value: i32) => 0 - value) == [3, 2, 2, 2, 1, 1]"),
        ("sort_by", "std.collections.sort_by(items, (a: i32, b: i32) => a.compare(b)) == [1, 1, 2, 2, 2, 3]"),
        ("stable_sort_by", "std.collections.stable_sort_by(items, (a: i32, b: i32) => a.compare(b)) == [1, 1, 2, 2, 2, 3]"),
        ("top_k", "std.collections.top_k(items.iter(), 3) == [3, 2, 2]"),
        ("bottom_k", "std.collections.bottom_k(items.iter(), 3) == [1, 1, 2]"),
    ]
    rng = random.Random(90210)
    values = list(range(48))
    rng.shuffle(values)
    samples = [values, list(range(47, -1, -1)), [1, 2, 7, 8, 9, 3, 4, 5, 0, 6], [rng.randrange(12) for _ in range(48)]]
    for values in samples:
        checks.append(("natural stable merge sorting", f"std.collections.stable_sort({array(values)}) == {array(sorted(values))}"))
        checks.append(("natural merge sorting", f"std.collections.sort({array(values)}) == {array(sorted(values))}"))
    for seed in [0, 7, 123]:
        value = 14695981039346656037 ^ seed
        for byte in "Etas-界-output".encode(): value = ((value ^ byte) * 1099511628211) % 2**64
        checks.append(("FNV profile", f'std.collections.Hasher.new({seed}).write("Etas-界-output".utf8).finish().bits == {value}'))
    return setup, checks


def float_checks(work):
    # Rust Display and parsing provide an independent shortest-decimal oracle.
    oracle = work / "float_oracle.rs"
    oracle.write_text('''use std::io::{self, BufRead};
fn main() {
    for line in io::stdin().lock().lines() {
        let line = line.unwrap();
        let (kind, raw) = line.split_once(' ').unwrap();
        let bits: u64 = raw.parse().unwrap();
        if kind == "32" {
            let text = f32::from_bits(bits as u32).to_string();
            println!("32 {} {}", bits, text);
        } else {
            let text = f64::from_bits(bits).to_string();
            println!("64 {} {}", bits, text);
        }
    }
}
''')
    executable = work / "float_oracle"
    subprocess.run(["rustc", "--edition=2021", "-O", str(oracle), "-o", str(executable)], check=True)
    rng = random.Random(2309)
    records = []
    for kind, fraction, exponent in [(32, 23, 8), (64, 52, 11)]:
        for _ in range(128):
            bits = (rng.randrange(2) << (kind - 1)) | (rng.randrange(1, 2**exponent - 1) << fraction) | rng.getrandbits(fraction)
            records.append(f"{kind} {bits}")
    output = subprocess.run([str(executable)], input="\n".join(records) + "\n", text=True, capture_output=True, check=True).stdout
    suites = {"float32": [], "float64": []}
    for line in output.splitlines():
        kind, bits, expected = line.split(" ", 2)
        suites[f"float{kind}"].extend([
            (f"Ryu f{kind} bits={bits}", f"std.text.from_f{kind}(f{kind} {{ bits= {bits} }}) == {literal(expected)}"),
            (f"Eisel-Lemire f{kind} bits={bits}", f"std.text.parse_f{kind}({literal(expected)}).unwrap().bits == {bits}"),
        ])
    return suites


def unicode_checks():
    data = Path(__file__).resolve().parents[2] / "etas_builtin/tests/unicode_data"
    rows = []
    for line in (data / "NormalizationTest.txt").read_text().splitlines():
        line = line.split("#", 1)[0].strip()
        if not line or line.startswith("@"): continue
        rows.append(["".join(chr(int(code, 16)) for code in field.split()) for field in line.split(";")[:5]])
    rng = random.Random(1700)
    samples = rows[:8] + rng.sample(rows[8:], 40)
    checks = []
    for source, *forms in samples:
        for form, expected in zip(["NFC", "NFD", "NFKC", "NFKD"], forms):
            args = f"{literal(source)}, std.text.unicode.{form}"
            checks.append((f"Unicode 17 {form}", f"std.text.unicode.normalize({args}) == {literal(expected)}"))
            checks.append((f"Quick_Check {form}", f"std.text.unicode.is_normalized({args}) == {str(source == expected).lower()}"))
            checks.append((f"normalized {form}", f"std.text.unicode.is_normalized({literal(expected)}, std.text.unicode.{form})"))
    rows = []
    for line in (data / "GraphemeBreakTest.txt").read_text().splitlines():
        tokens = line.split("#", 1)[0].split()
        if not tokens: continue
        clusters = []; chunk = ""
        for token in tokens:
            if token == "÷":
                if chunk: clusters.append(chunk); chunk = ""
            elif token != "×": chunk += chr(int(token, 16))
        if chunk: clusters.append(chunk)
        rows.append(clusters)
    for clusters in rows[:8] + rng.sample(rows[8:], 24):
        source = "".join(clusters)
        checks.extend([
            ("Unicode 17 grapheme segmentation", f"std.text.unicode.graphemes({literal(source)}).collect() == {array(clusters)}"),
            ("grapheme reverse", f"std.text.unicode.reverse_graphemes({literal(source)}) == {literal(''.join(reversed(clusters)))}"),
        ])
    checks.extend([
        ("grapheme slice", 'std.text.unicode.grapheme_slice("Aé界tail", [1, 3)) == "é界"'),
        ("grapheme byte indices", 'std.text.unicode.grapheme_indices("Aé界").collect() == [(0, "A"), (1, "é"), (3, "界")]'),
    ])
    return "", checks


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--sysroot", type=Path, required=True)
    parser.add_argument("--work-dir", type=Path)
    args = parser.parse_args()
    work = args.work_dir or Path(tempfile.mkdtemp(prefix="etas-algorithms-"))
    work.mkdir(parents=True, exist_ok=True)
    print(f"Validation files: {work}", flush=True)
    total = 0
    for name, factory in [("text", text_checks), ("collections", collection_checks)]:
        setup, checks = factory()
        total += run_suite(args.binary.resolve(), args.sysroot.resolve(), work, name, setup, checks)
    for name, checks in float_checks(work).items():
        total += run_suite(args.binary.resolve(), args.sysroot.resolve(), work, name, "", checks)
    setup, checks = unicode_checks()
    total += run_suite(args.binary.resolve(), args.sysroot.resolve(), work, "unicode", setup, checks)
    print(f"PASS all {total} source SDK checks", flush=True)


if __name__ == "__main__":
    main()
