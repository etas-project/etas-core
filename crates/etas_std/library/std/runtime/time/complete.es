module std.runtime.time;

public type Time = { unix_millis: i128 };

impl Time {
    // Bias signed timestamps into the complete u128 order. Durations can
    // exceed i128::MAX while the resulting timestamp still remains valid.
    flow order_key(self) -> u128 ![] {
        let sign: u128 = 170141183460469231731687303715884105728;
        let bits = self.unix_millis.bits;
        return if bits < sign { bits + sign } else { bits - sign };
    }

    flow from_order_key(key: u128) -> Time ![] {
        let sign: u128 = 170141183460469231731687303715884105728;
        let bits = if key < sign { key + sign } else { key - sign };
        return Time { unix_millis= i128 { bits= bits } };
    }

    public flow checked_add(self, duration: Duration) -> Result<Time, TimeError> ![] {
        return match std.math.checked_add(self.order_key(), duration.millis) {
            Ok(key) => Ok(Time.from_order_key(key)),
            Err(_) => Err(TimeError.Overflow),
        };
    }

    public flow checked_sub(self, duration: Duration) -> Result<Time, TimeError> ![] {
        return match std.math.checked_sub(self.order_key(), duration.millis) {
            Ok(key) => Ok(Time.from_order_key(key)),
            Err(_) => Err(TimeError.Overflow),
        };
    }

    public flow since(self, earlier: Time) -> Result<Duration, TimeError> ![] {
        if self.unix_millis < earlier.unix_millis { return Err(TimeError.InvalidOrder); }
        return Ok(Duration { millis= self.order_key() - earlier.order_key() });
    }
}

// Observe wall time through the caller's Handler or the runtime clock.
public flow now() -> Time ![Error<TimeError>] {
    return perform std.effects.Clock.now();
}

// A zero duration completes locally; it does not promise a scheduler yield.
public flow sleep(duration: Duration) -> unit ![Error<TimeError>] {
    if duration.millis == 0 { return (); }
    perform std.effects.Clock.sleep(duration);
}
