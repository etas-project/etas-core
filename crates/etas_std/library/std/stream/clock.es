module std.stream;

// Read only the execution clock, in milliseconds, without performing stream
// reduction or changing authority. The VM supplies this clock across resume.
type StreamClock = {};

impl StreamClock {
    @std.role("std.stream.StreamClock.now")
    flow now() -> u128 ![];
}
