module std.http.codec;

public import std.http.codec.HttpCodecFailureKind.ConflictingContentLength;
public import std.http.codec.HttpCodecFailureKind.ConflictingMessageFraming;
public import std.http.codec.HttpCodecFailureKind.ForbiddenResponseBody;
public import std.http.codec.HttpCodecFailureKind.InvalidChunkExtension;
public import std.http.codec.HttpCodecFailureKind.InvalidChunkSize;
public import std.http.codec.HttpCodecFailureKind.InvalidChunkTerminator;
public import std.http.codec.HttpCodecFailureKind.InvalidContentLength;
public import std.http.codec.HttpCodecFailureKind.InvalidHeader;
public import std.http.codec.HttpCodecFailureKind.InvalidLineEnding;
public import std.http.codec.HttpCodecFailureKind.InvalidStatusCode;
public import std.http.codec.HttpCodecFailureKind.InvalidStatusLine;
public import std.http.codec.HttpCodecFailureKind.InvalidTrailer;
public import std.http.codec.HttpCodecFailureKind.LimitExceeded;
public import std.http.codec.HttpCodecError.MalformedMessage;
public import std.http.codec.HttpCodecFailureKind.UnexpectedEof;
public import std.http.codec.HttpCodecFailureKind.UnsupportedHttpVersion;
public import std.http.codec.HttpCodecFailureKind.UnsupportedTransferEncoding;
public import std.http.codec.HttpDecodeStep.{NeedMore, Complete, Malformed};
// HTTP codec support type.
public enum HttpCodecError {
    MalformedMessage,
}
// HTTP codec support type.
public enum HttpCodecFailureKind {
    ConflictingContentLength,
    ConflictingMessageFraming,
    ForbiddenResponseBody,
    InvalidChunkExtension,
    InvalidChunkSize,
    InvalidChunkTerminator,
    InvalidContentLength,
    InvalidHeader,
    InvalidLineEnding,
    InvalidStatusCode,
    InvalidStatusLine,
    InvalidTrailer,
    LimitExceeded,
    UnexpectedEof,
    UnsupportedHttpVersion,
    UnsupportedTransferEncoding,
}
// HTTP codec support type.
public type HttpDecodeFailure = { kind: std.http.codec.HttpCodecFailureKind, offset: usize };
// Closed incremental HTTP decode result.
public enum HttpDecodeStep<T> {
    Complete(T, usize),
    Malformed(std.http.codec.HttpDecodeFailure),
    NeedMore,
}
// HTTP codec support type.
public type HttpHeader = { name: string, value: string };
// HTTP codec support type.
public type HttpWireRequest = { method: string, target: string, version: string, headers: std.collections.List<std.http.codec.HttpHeader>, body: bytes };
// HTTP codec support type.
public type HttpWireResponse = { head: std.http.codec.HttpWireResponseHead, body: bytes };
// HTTP codec support type.
public type HttpWireResponseHead = { version: string, status: i32, reason: string, headers: std.collections.List<std.http.codec.HttpHeader> };
