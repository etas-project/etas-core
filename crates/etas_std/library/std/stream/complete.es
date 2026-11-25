module std.stream;

public import std.stream.StreamRead.{Data, Eof};
public import std.stream.StreamError.{Busy, Cancelled, Closed, Host, Interrupted, LimitExceeded, TimedOut, UnexpectedEof, UnsupportedOperation};

// Bounded stream operation configuration.
public type ByteLimit = { bytes: usize };

// Standard byte-stream marker spec.
public spec ByteStream;

// Typed stream operation failure.
public enum StreamError {
    Busy,
    Cancelled,
    Closed,
    Host(string),
    Interrupted,
    LimitExceeded,
    TimedOut,
    UnexpectedEof,
    UnsupportedOperation,
}

// A byte chunk or ordinary end of stream.
public enum StreamRead {
    Data(bytes),
    Eof,
}

// Bounded stream operation configuration.
public alias Timeout = std.runtime.time.Duration;


public flow close<S ~ ByteStream>(stream: S) -> unit ![Error<StreamError>] {
    perform std.effects.Stream.close<S>(stream);
}

public flow flush<S ~ ByteStream>(stream: S, timeout: Option<Timeout> = None) -> unit ![Error<StreamError>] {
    perform std.effects.Stream.flush<S>(stream, timeout);
}

public flow read<S ~ ByteStream>(stream: S, max_bytes: usize, timeout: Option<Timeout> = None) -> StreamRead ![Error<StreamError>] {
    return perform std.effects.Stream.read<S>(stream, max_bytes, timeout);
}

type ReadDeadline = { private start: u128, private duration: Option<Timeout> };

impl ReadDeadline {
    flow new(timeout: Option<Timeout>) -> ReadDeadline ![] {
        let start = match timeout { Some(_) => StreamClock.now(), None => 0 };
        return ReadDeadline { start= start, duration= timeout };
    }

    flow remaining(self) -> Option<Timeout> ![Error<StreamError>] {
        return match self.duration {
            None => None,
            Some(duration) => {
                let elapsed = StreamClock.now() - self.start;
                if elapsed >= duration.millis {
                    perform Error.raise<StreamError>(StreamError.TimedOut);
                }
                Some(std.runtime.time.Duration.from_millis(duration.millis - elapsed))
            },
        };
    }
}

public flow read_exact<S ~ ByteStream>(stream: S, count: usize, timeout: Option<Timeout> = None) -> bytes ![Error<StreamError>] {
    let (input, owner) = ReadScope.enter(stream);
    var body = bytes { data= [] };
    if count == 0 {
        perform std.effects.Stream.read<S>(input, 0, timeout);
        return body;
    }
    let deadline = ReadDeadline.new(timeout);
    while body.length < count {
        let remaining = count - body.length;
        let requested = if remaining > 8192 { 8192 } else { remaining };
        match perform std.effects.Stream.read<S>(input, requested, deadline.remaining()) {
            StreamRead.Eof => perform Error.raise<StreamError>(StreamError.UnexpectedEof),
            StreamRead.Data(chunk) => { body = body + chunk; },
        };
        deadline.remaining();
    }
    return body;
}

public flow read_until_limit<S ~ ByteStream>(stream: S, limit: ByteLimit, timeout: Option<Timeout> = None) -> bytes ![Error<StreamError>] {
    let (input, owner) = ReadScope.enter(stream);
    let maximum = limit.bytes;
    let deadline = ReadDeadline.new(timeout);
    var body = bytes { data= [] };
    while true {
        let remaining_timeout = deadline.remaining();
        let remaining = maximum - body.length;
        // One extra byte distinguishes an exactly full stream from truncation.
        let requested = if remaining >= 8192 { 8192 } else { remaining + 1 };
        let part = perform std.effects.Stream.read<S>(input, requested, remaining_timeout);
        deadline.remaining();
        match part {
            StreamRead.Eof => { return body; },
            StreamRead.Data(chunk) => {
                if chunk.length > remaining { perform Error.raise<StreamError>(StreamError.LimitExceeded); }
                body = body + chunk;
            },
        };
    }
}

public flow write_all<S ~ ByteStream>(stream: S, body: bytes, timeout: Option<Timeout> = None) -> unit ![Error<StreamError>] {
    perform std.effects.Stream.write<S>(stream, body, timeout);
}
