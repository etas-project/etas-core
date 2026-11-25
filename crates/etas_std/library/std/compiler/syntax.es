module std.compiler;

// Parsing creates an immutable association between source, tokens and AST.
// That association is a compiler mechanism; formatting is an Etas algorithm.
@std.role("std.compiler.parse_body")
public flow parse_body(text: string) -> Result<SyntaxBlock, ParseError> ![];

public flow format(body: SyntaxBlock) -> string ![] {
    var formatter = BodyFormatter {
        buffer= FormatBuffer { parts= [], size= 0, maximum= body.format_limit },
        indent= 0,
        previous= None,
    };
    var index: usize = 0;
    var current: Option<SyntaxToken> = None;
    while index < body.tokens.length {
        let next = body.tokens.get(index).unwrap();
        index = index + 1;
        if next.kind != SyntaxTokenKind.Trivia {
            match current {
                Some(token) => formatter.token(token, Some(next)),
                None => {},
            };
            if next.kind == SyntaxTokenKind.Eof {
                var buffer = formatter.buffer;
                return buffer.finish();
            }
            current = Some(next);
        }
    }
    match current {
        Some(token) => formatter.token(token, None),
        None => {},
    };
    var buffer = formatter.buffer;
    return buffer.finish();
}
