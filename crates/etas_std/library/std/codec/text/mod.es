module std.codec.text;

public import std.codec.text.MalformedInput.{Strict, Replace};
public import std.codec.text.TextCodecError.InvalidUtf8;

public enum MalformedInput { Strict, Replace, }
public enum TextCodecError { InvalidUtf8, }

@std.binding("std.codec.text.utf8_encode")
public flow utf8_encode(value: string) -> bytes ![] {}

@std.binding("std.codec.text.utf8_decode")
public flow utf8_decode(value: bytes, malformed: MalformedInput) -> Result<string, TextCodecError> ![] {}
