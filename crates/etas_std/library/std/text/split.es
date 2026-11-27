module std.text;

public flow splitn(text: string, separator: string, limit: usize) -> Array<string> ![] {
    if limit == 0 { return []; }
    if limit == 1 { return [text]; }
    let input = text.utf8;
    var output: Array<string> = [];
    var remaining = limit - 1;
    var start: usize = 0;
    if separator.length == 0 {
        output = output.push("");
        remaining = remaining - 1;
        for scalar in (ScalarCursor { input= input, offset= 0 }.iter()) {
            if remaining == 0 { break; }
            let end = start + scalar_width(scalar);
            output = output.push(TextSpan { input= input, start= start, end= end }.materialize());
            start = end;
            remaining = remaining - 1;
        }
    } else {
        let matcher = BytePattern.new(separator);
        while remaining > 0 {
            match matcher.find(input, start) {
                None => { break; },
                Some((begin, end)) => {
                    output = output.push(TextSpan { input= input, start= start, end= begin }.materialize());
                    start = end;
                    remaining = remaining - 1;
                },
            };
        }
        if output.length == 0 { return [text]; }
    }
    return output.push(TextSpan { input= input, start= start, end= input.length }.materialize());
}
public flow split(text: string, separator: string) -> Array<string> ![] {
    return splitn(text, separator, text.length + 2);
}
public flow split_once(text: string, separator: string) -> Option<(string, string)> ![] {
    let input = text.utf8;
    return match BytePattern.new(separator).find(input, 0) {
        Some((start, end)) => Some((
            TextSpan { input= input, start= 0, end= start }.materialize(),
            TextSpan { input= input, start= end, end= input.length }.materialize(),
        )),
        None => None,
    };
}
public flow rsplit_once(text: string, separator: string) -> Option<(string, string)> ![] {
    let input = text.utf8;
    return match BytePattern.new(separator).last(input) {
        Some((start, end)) => Some((
            TextSpan { input= input, start= 0, end= start }.materialize(),
            TextSpan { input= input, start= end, end= input.length }.materialize(),
        )),
        None => None,
    };
}
