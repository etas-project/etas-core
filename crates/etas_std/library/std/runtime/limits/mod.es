module std.runtime.limits;

import std.runtime.budget.Money;
import std.runtime.time.Duration;

@std.role("std.runtime.limits.Limit")
public enum Limit {
    Iterations(usize),
    Tokens(usize),
    ContextTokens(usize),
    Cost(Money),
    WallTime(Duration),
    Attempts(usize),
    Concurrency(usize),
}

@std.binding("std.runtime.limits.Iterations")
public flow Iterations(value: usize) -> Limit ![] {}

@std.binding("std.runtime.limits.Tokens")
public flow Tokens(value: usize) -> Limit ![] {}

@std.binding("std.runtime.limits.ContextTokens")
public flow ContextTokens(value: usize) -> Limit ![] {}

@std.binding("std.runtime.limits.Cost")
public flow Cost(value: Money) -> Limit ![] {}

@std.binding("std.runtime.limits.WallTime")
public flow WallTime(value: Duration) -> Limit ![] {}

@std.binding("std.runtime.limits.Attempts")
public flow Attempts(value: usize) -> Limit ![] {}

@std.binding("std.runtime.limits.Concurrency")
public flow Concurrency(value: usize) -> Limit ![] {}
