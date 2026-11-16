module std.runtime.budget;

@std.role("std.runtime.budget.Money")
public type Money;

// The runtime creates the opaque monetary value; source owns its public API.
@std.binding("std.runtime.budget.usd")
public flow usd(amount: f64) -> Money ![] {}
