module std.task;

// Public declarations migrated from the legacy API inventory.
// Maintained as Etas source; native bindings retain existing execution during migration.

// Run every homogeneous job and retain an input-ordered Result for each branch.
@std.binding("std.task.collect")
public flow collect<T, X, effect E>(arg0: std.collections.Array<() -> T ![Error<X>, E]>) -> std.collections.Array<std.result.Result<T, X>> ![E] {}

// Run fixed or homogeneous jobs concurrently and preserve source result order.
@std.binding("std.task.join")
public flow join<A, B, effect E>(arg0: (() -> A ![E], () -> B ![E])) -> (A, B) ![E] {}

// Alternate public signature for the same native operation.
@std.binding("std.task.join")
public flow join<A, B, C, effect E>(arg0: (() -> A ![E], () -> B ![E], () -> C ![E])) -> (A, B, C) ![E] {}

// Alternate public signature for the same native operation.
@std.binding("std.task.join")
public flow join<A, B, C, D, effect E>(arg0: (() -> A ![E], () -> B ![E], () -> C ![E], () -> D ![E])) -> (A, B, C, D) ![E] {}

// Alternate public signature for the same native operation.
@std.binding("std.task.join")
public flow join<T, effect E>(arg0: std.collections.Array<() -> T ![E]>) -> std.collections.Array<T> ![E] {}

// Map an array concurrently with stable input-to-output ordering.
@std.binding("std.task.map_concurrent")
public flow map_concurrent<A, B, effect E>(arg0: std.collections.Array<A>, arg1: (A) -> B ![E]) -> std.collections.Array<B> ![E] {}

// Map an array concurrently and retain an input-ordered Result for every item.
@std.binding("std.task.map_concurrent_collect")
public flow map_concurrent_collect<A, B, X, effect E>(arg0: std.collections.Array<A>, arg1: (A) -> B ![Error<X>, E]) -> std.collections.Array<std.result.Result<B, X>> ![E] {}

// Return the first successful branch and propagate a deterministic error if all fail.
@std.binding("std.task.race")
public flow race<T, X, effect E>(arg0: std.collections.Array<() -> T ![Error<X>, E]>) -> T ![Error<X>, E] {}

// Run homogeneous jobs fail-fast and return the primary typed error as Result.
@std.binding("std.task.try_join")
public flow try_join<T, X, effect E>(arg0: std.collections.Array<() -> T ![Error<X>, E]>) -> std.result.Result<std.collections.Array<T>, X> ![E] {}
