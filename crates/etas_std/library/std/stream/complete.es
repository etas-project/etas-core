module std.stream;

public import std.stream.StreamRead.{Data, Eof};
public import std.stream.StreamError.{Cancelled, Closed, Host, Interrupted, LimitExceeded, TimedOut};

// Bounded stream operation configuration.
public type ByteLimit = { bytes: i32 };

// Standard byte-stream marker spec.
public spec ByteStream;

// Typed stream operation failure.
public enum StreamError {
    Cancelled,
    Closed,
    Host(string),
    Interrupted,
    LimitExceeded,
    TimedOut,
}

// A byte chunk or ordinary end of stream.
public enum StreamRead {
    Data(bytes),
    Eof,
}

// Bounded stream operation configuration.
public type Timeout = { ms: i32 };


public flow close<S ~ ByteStream>(stream: S) -> unit ![Error<StreamError>] {
    perform std.effects.Stream.close<S>(stream);
}

public flow flush<S ~ ByteStream>(stream: S) -> unit ![Error<StreamError>] {
    perform std.effects.Stream.flush<S>(stream);
}

public flow read<S ~ ByteStream>(stream: S, max_bytes: usize, timeout: Option<Timeout>) -> StreamRead ![Error<StreamError>] {
    match timeout {
        Some(value) => {
            if value.ms < 0 { std.core.abort("Timeout must be nonnegative"); }
        },
        None => {},
    };
    return perform std.effects.Stream.read<S>(stream, max_bytes, timeout);
}

public flow read_until_limit<S ~ ByteStream>(stream: S, limit: ByteLimit, timeout: Option<Timeout>) -> bytes ![Error<StreamError>] {
    let maximum = match limit.bytes::std.core.Index.to_usize() {
        Some(value) => value,
        None => std.core.abort("ByteLimit must be nonnegative"),
    };
    let deadline = match timeout {
        None => None,
        Some(value) => {
            if value.ms < 0 { std.core.abort("Timeout must be nonnegative"); }
            Some(StreamClock.now() + value.ms.bits)
        },
    };
    var body = bytes { data= [] };
    while true limit Iterations(18446744073709551615) {
        let remaining_timeout = match deadline {
            None => None,
            Some(end) => {
                let now = StreamClock.now();
                if now >= end { perform Error.raise<StreamError>(StreamError.TimedOut); }
                Some(Timeout { ms= i32 { bits= end - now } })
            },
        };
        let remaining = maximum - body.length;
        // One extra byte distinguishes an exactly full stream from truncation.
        let requested = if remaining >= 8192 { 8192 } else { remaining + 1 };
        let part = perform std.effects.Stream.read<S>(stream, requested, remaining_timeout);
        match deadline {
            Some(end) => {
                if StreamClock.now() >= end { perform Error.raise<StreamError>(StreamError.TimedOut); }
            },
            None => {},
        };
        match part {
            StreamRead.Eof => { return body; },
            StreamRead.Data(chunk) => {
                if chunk.length > remaining { perform Error.raise<StreamError>(StreamError.LimitExceeded); }
                body = body + chunk;
            },
        };
    }
}

public flow write_all<S ~ ByteStream>(stream: S, body: bytes) -> unit ![Error<StreamError>] {
    perform std.effects.Stream.write<S>(stream, body);
}
