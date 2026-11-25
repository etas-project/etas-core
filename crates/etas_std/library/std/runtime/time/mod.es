module std.runtime.time;

public type Duration = { millis: u128 };

public enum TimeError {
    Overflow,
    InvalidOrder,
    ClockMismatch,
    Unavailable,
    Host(string),
}

impl Duration {
    public flow from_millis(value: u128) -> Duration ![] {
        return Duration { millis= value };
    }

    public flow from_seconds(value: u128) -> Result<Duration, TimeError> ![] {
        return match std.math.checked_mul(value, 1000) {
            Ok(millis) => Ok(Duration { millis= millis }),
            Err(_) => Err(TimeError.Overflow),
        };
    }

    public flow as_millis(self) -> u128 ![] {
        return self.millis;
    }

    public flow as_seconds(self) -> u128 ![] {
        return self.millis / 1000;
    }

    public flow checked_add(self, other: Duration) -> Result<Duration, TimeError> ![] {
        return match std.math.checked_add(self.millis, other.millis) {
            Ok(millis) => Ok(Duration { millis= millis }),
            Err(_) => Err(TimeError.Overflow),
        };
    }

    public flow checked_sub(self, other: Duration) -> Result<Duration, TimeError> ![] {
        if self.millis < other.millis { return Err(TimeError.InvalidOrder); }
        return Ok(Duration { millis= self.millis - other.millis });
    }

    public flow checked_mul(self, factor: u128) -> Result<Duration, TimeError> ![] {
        return match std.math.checked_mul(self.millis, factor) {
            Ok(millis) => Ok(Duration { millis= millis }),
            Err(_) => Err(TimeError.Overflow),
        };
    }
}
