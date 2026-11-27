module std.text;

public flow ascii_lowercase(text: string) -> string ![] {
    var output: Array<char> = [];
    var changed = false;
    for scalar in chars(text) {
        let code = scalar.bits;
        if code >= 65 && code <= 90 {
            output = output.push(char { bits= code + 32 });
            changed = true;
        } else { output = output.push(scalar); }
    }
    if !changed { return text; }
    return string { scalars= output };
}
public flow ascii_uppercase(text: string) -> string ![] {
    var output: Array<char> = [];
    var changed = false;
    for scalar in chars(text) {
        let code = scalar.bits;
        if code >= 97 && code <= 122 {
            output = output.push(char { bits= code - 32 });
            changed = true;
        } else { output = output.push(scalar); }
    }
    if !changed { return text; }
    return string { scalars= output };
}
public flow eq_ignore_ascii_case(left: string, right: string) -> bool ![] {
    if left.length != right.length { return false; }
    let a = left.utf8;
    let b = right.utf8;
    var index: usize = 0;
    while index < a.length {
        let x = a.get(index).unwrap();
        let y = b.get(index).unwrap();
        let folded_x = if x >= 65 && x <= 90 { x + 32 } else { x };
        let folded_y = if y >= 65 && y <= 90 { y + 32 } else { y };
        if folded_x != folded_y { return false; }
        index = index + 1;
    }
    return true;
}
