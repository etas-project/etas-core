module std.security.declassify;

// Public declarations migrated from the legacy API inventory.
// Maintained as Etas source; native bindings retain existing execution during migration.

// Security declassification support descriptor.
public flow declassify<T>(arg0: T) -> T ![Secret];

// Security declassification support descriptor.
public flow sanitize<T>(arg0: T) -> T ![Secret];
