module std.security.trust;

// Public declarations migrated from the legacy API inventory.
// Maintained as Etas source; native bindings retain existing execution during migration.

// Trust wrapper declaration.
@std.role("std.security.trust.Public")
public type Public<T>;

// Trust wrapper declaration.
@std.role("std.security.trust.Sanitized")
public type Sanitized<T>;

// Trust wrapper declaration.
@std.role("std.security.trust.Secret")
public type Secret<T>;

// Trust wrapper declaration.
@std.role("std.security.trust.Trusted")
public type Trusted<T>;

// Trust wrapper declaration.
@std.role("std.security.trust.Untrusted")
public type Untrusted<T>;
