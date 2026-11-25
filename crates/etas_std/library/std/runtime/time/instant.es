module std.runtime.time;

// The runtime alone creates the sealed clock-domain witness. It is not data
// that JSON, storage or an ordinary value decoder can recreate.
type ClockDomain;

public type Instant = {
    private clock: ClockDomain,
    private domain: string,
    private millis: u128,
};

impl Instant {
    public flow since(self, earlier: Instant) -> Result<Duration, TimeError> ![] {
        if self.domain != earlier.domain { return Err(TimeError.ClockMismatch); }
        if self.millis < earlier.millis { return Err(TimeError.InvalidOrder); }
        return Ok(Duration { millis= self.millis - earlier.millis });
    }
}

public flow monotonic() -> Instant ![Error<TimeError>] {
    return perform std.effects.Clock.monotonic();
}

public flow elapsed(start: Instant) -> Duration ![Error<TimeError>] {
    let current = monotonic();
    return match current.since(start) {
        Ok(duration) => duration,
        Err(error) => perform Error.raise<TimeError>(error),
    };
}
