module std.effects;

// External cryptographic entropy. Source handlers may replace this default.
public effect Random {
    @default_handler
    action entropy(count: usize) -> bytes ![Error<std.random.RandomError>];
}
