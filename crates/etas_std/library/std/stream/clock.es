module std.stream;

// Monotonic execution milliseconds for local stream deadlines. The VM retains
// these observations across resume; this is not a wall timestamp.
type StreamClock = {};

impl StreamClock {
    @std.role("std.stream.StreamClock.now")
    flow now() -> u128 ![];
}
