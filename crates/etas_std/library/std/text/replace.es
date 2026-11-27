module std.text;

public flow replace_n(text: string, pattern: string, replacement: string, limit: usize) -> string ![] {
    if limit == 0 { return text; }
    let input = text.utf8;
    var output = TextBuilder.new();
    var remaining = limit;
    var start: usize = 0;
    if pattern.length == 0 {
        output.append(replacement);
        remaining = remaining - 1;
        for scalar in (ScalarCursor { input= input, offset= 0 }.iter()) {
            if remaining == 0 { break; }
            let end = start + scalar_width(scalar);
            output.append(TextSpan { input= input, start= start, end= end }.materialize());
            output.append(replacement);
            start = end;
            remaining = remaining - 1;
        }
    } else {
        let matcher = BytePattern.new(pattern);
        while remaining > 0 {
            match matcher.find(input, start) {
                None => { break; },
                Some((begin, end)) => {
                    output.append(TextSpan { input= input, start= start, end= begin }.materialize());
                    output.append(replacement);
                    start = end;
                    remaining = remaining - 1;
                },
            };
        }
        if remaining == limit { return text; }
    }
    output.append(TextSpan { input= input, start= start, end= input.length }.materialize());
    return output.build();
}
public flow replace(text: string, pattern: string, replacement: string) -> string ![] {
    return replace_n(text, pattern, replacement, text.length + 1);
}
