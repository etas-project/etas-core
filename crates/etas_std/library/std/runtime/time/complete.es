module std.runtime.time;

// Runtime time support type.
public type Time = { unix_millis: u128 };

// Read the runtime clock through the future runtime boundary.

public flow now() -> Time ![std.effects.Time] {
    return perform std.effects.Clock.now();
}
