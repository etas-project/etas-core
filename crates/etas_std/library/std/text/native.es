module std.text;

public flow char_count(value: string) -> usize ![] {
    var count: usize = 0;
    for scalar in chars(value) { count = count + 1; }
    return count;
}
public flow is_empty(value: string) -> bool ![] { return value.length == 0; }

public flow trim_start(value: string) -> string ![] {
    let input = value.utf8;
    var start: usize = 0;
    for scalar in (ScalarCursor { input= input, offset= 0 }.iter()) {
        if !scalar.is_whitespace() { break; }
        start = start + scalar_width(scalar);
    }
    if start == 0 { return value; }
    return TextSpan { input= input, start= start, end= input.length }.materialize();
}
public flow trim_end(value: string) -> string ![] {
    let input = value.utf8;
    var offset: usize = 0;
    var end: usize = 0;
    for scalar in (ScalarCursor { input= input, offset= 0 }.iter()) {
        offset = offset + scalar_width(scalar);
        if !scalar.is_whitespace() { end = offset; }
    }
    if end == input.length { return value; }
    return TextSpan { input= input, start= 0, end= end }.materialize();
}
public flow trim(value: string) -> string ![] {
    let input = value.utf8;
    var offset: usize = 0;
    var start: Option<usize> = None;
    var end: usize = 0;
    for scalar in (ScalarCursor { input= input, offset= 0 }.iter()) {
        let next = offset + scalar_width(scalar);
        if !scalar.is_whitespace() {
            if start.is_none() { start = Some(offset); }
            end = next;
        }
        offset = next;
    }
    let begin = match start { None => { return ""; }, Some(at) => at };
    if begin == 0 && end == input.length { return value; }
    return TextSpan { input= input, start= begin, end= end }.materialize();
}
