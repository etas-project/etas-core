module std.text;

public import std.text.TextError.{

    InvalidIndex, InvalidRange, NotCharBoundary, ParseFailure,
    NumericOverflow, LengthOverflow, InvalidFormat,
};

public enum TextError {
    InvalidIndex,
    InvalidRange,
    NotCharBoundary,
    ParseFailure,
    NumericOverflow,
    LengthOverflow,
    InvalidFormat,
}
