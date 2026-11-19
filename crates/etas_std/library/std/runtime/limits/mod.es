module std.runtime.limits;

import std.runtime.budget.Money;
import std.runtime.time.Duration;
public import std.runtime.limits.Limit.{Iterations, Tokens, ContextTokens, Cost, WallTime, Attempts, Concurrency};


public enum Limit {
    Iterations(usize),
    Tokens(usize),
    ContextTokens(usize),
    Cost(Money),
    WallTime(Duration),
    Attempts(usize),
    Concurrency(usize),
}
