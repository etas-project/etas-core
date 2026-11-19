module std.host.path;

// Paths and patterns are data. Sandbox authority belongs to the operation.
public type Path = { text: string };
public type PathPattern = { pattern: string };
