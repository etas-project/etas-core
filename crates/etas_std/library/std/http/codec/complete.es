module std.http.codec;

// Public declarations migrated from the legacy API inventory.
// Maintained as Etas source; native bindings retain existing execution during migration.

public import std.http.codec.HttpCodecFailureKind.ConflictingContentLength;

public import std.http.codec.HttpCodecFailureKind.ConflictingMessageFraming;

public import std.http.codec.HttpCodecFailureKind.ForbiddenResponseBody;

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

// Decode one chunk size and validate extensions within a byte limit including CRLF, without consuming data.
@std.binding("std.http.codec.decode_chunk_size_line_prefix")
public flow decode_chunk_size_line_prefix(arg0: bytes, arg1: usize) -> std.http.codec.HttpDecodeStep<u64> ![] {}

// Decode an HTTP response head and body from bytes.
@std.binding("std.http.codec.decode_response")
public flow decode_response(arg0: bytes) -> std.result.Result<std.http.codec.HttpWireResponse, std.http.codec.HttpCodecError> ![] {}

// Decode an HTTP response head from bytes.
@std.binding("std.http.codec.decode_response_head")
public flow decode_response_head(arg0: bytes) -> std.result.Result<std.http.codec.HttpWireResponseHead, std.http.codec.HttpCodecError> ![] {}

// Incrementally decode an HTTP response head without conflating incomplete input and malformed input.
@std.binding("std.http.codec.decode_response_head_incremental")
public flow decode_response_head_incremental(arg0: bytes, arg1: bool) -> std.http.codec.HttpDecodeStep<std.http.codec.HttpWireResponseHead> ![] {}

// Decode one response head within a byte limit including its terminator, preserving following bytes.
@std.binding("std.http.codec.decode_response_head_prefix")
public flow decode_response_head_prefix(arg0: bytes, arg1: usize) -> std.http.codec.HttpDecodeStep<std.http.codec.HttpWireResponseHead> ![] {}

// Incrementally decode a framed HTTP response and report consumed input bytes.
@std.binding("std.http.codec.decode_response_incremental")
public flow decode_response_incremental(arg0: bytes, arg1: bool) -> std.http.codec.HttpDecodeStep<std.http.codec.HttpWireResponse> ![] {}

// Encode a structured HTTP request head/body into bytes.
@std.binding("std.http.codec.encode_request")
public flow encode_request(arg0: std.http.codec.HttpWireRequest) -> std.result.Result<bytes, std.http.codec.HttpCodecError> ![] {}

public import std.http.codec.HttpDecodeStep.{NeedMore, Complete, Malformed};
