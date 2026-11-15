module std.compiler;

import std.compiler.SyntaxBlock;

// Syntax processing uses the compiler's parser and formatter. The public
// declarations and the data returned for errors belong to this source package.
@std.binding("std.compiler.parse_body")
public flow parse_body(text: string) -> Result<SyntaxBlock, ParseError> ![] {}

@std.binding("std.compiler.format")
public flow format(body: SyntaxBlock) -> string ![] {}
