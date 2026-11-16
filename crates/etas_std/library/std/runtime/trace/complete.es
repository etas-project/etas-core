module std.runtime.trace;

// Public declarations migrated from the legacy API inventory.
// Maintained as Etas source; native bindings retain existing execution during migration.

@std.role("std.runtime.trace.Logical")
public let Logical: std.runtime.trace.TraceLabel;

// Trace support type.
public type TraceExport;

// Trace support type.
public type TraceLabel;

// Trace support type.
public type TraceRedaction;

// Trace label for a fused agent with virtual logical stages.
public flow VirtualStages(arg0: std.collections.Array<std.runtime.trace.TraceLabel>) -> std.runtime.trace.TraceLabel ![];
