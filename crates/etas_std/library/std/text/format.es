module __etas_std.text.format;

import std.collections.array_get;
import std.text.{InvalidFormat, TextError, chars, from_char, parse_usize};

flow format(template: string, arguments: Array<string>) -> Result<string, TextError> {
    var output = "";
    var index_text = "";
    var after_open = false;
    var after_close = false;
    var placeholder = false;

    for scalar in chars(template) limit Iterations(18446744073709551615) {
        if placeholder {
            if scalar == '}' {
                let index = match parse_usize(index_text) {
                    Ok(value) => value,
                    Err(_) => { return Err(InvalidFormat); },
                };
                match array_get(arguments, index) {
                    Some(value) => { output = output + value; },
                    None => { return Err(InvalidFormat); },
                }
                index_text = "";
                placeholder = false;
            } else {
                index_text = index_text + from_char(scalar);
            }
        } else if after_open {
            if scalar == '{' {
                output = output + "{";
            } else if scalar == '}' {
                return Err(InvalidFormat);
            } else {
                placeholder = true;
                index_text = from_char(scalar);
            }
            after_open = false;
        } else if after_close {
            if scalar != '}' {
                return Err(InvalidFormat);
            }
            output = output + "}";
            after_close = false;
        } else if scalar == '{' {
            after_open = true;
        } else if scalar == '}' {
            after_close = true;
        } else {
            output = output + from_char(scalar);
        }
    }

    if after_open || after_close || placeholder {
        return Err(InvalidFormat);
    }
    return Ok(output);
}
