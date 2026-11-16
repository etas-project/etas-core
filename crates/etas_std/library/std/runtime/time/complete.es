module std.runtime.time;

// Public declarations migrated from the legacy API inventory.
// Maintained as Etas source; native bindings retain existing execution during migration.

// Runtime time support type.
public type Time;

// Read the runtime clock through the future runtime boundary.
@std.binding("std.runtime.time.now")
public flow now() -> std.runtime.time.Time ![Time] {}
