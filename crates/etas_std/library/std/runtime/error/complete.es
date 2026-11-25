module std.runtime.error;

public import std.io.IOError;

// Standard runtime error support type.
public type BudgetExceeded = { resource: string, limit: u128, consumed: u128 };

// Standard runtime error support type.
public type EffectBoundaryViolation = { effect: string, boundary: string };

// Standard runtime error support type.
public type HumanRejected = { reason: Option<string> };


// Standard runtime error support type.
public type IndexError = { index: usize, length: usize };

// Standard runtime error support type.
public type MissingCitation = { claim: string };

// Standard runtime error support type.
public type PolicyDenied = { policy: string, reason: string };

// Standard runtime error support type.
public type PolicyViolation = { policy: string, message: string };

// Standard runtime error support type.
public type PromptInjectionRisk = { message: string };

// Standard runtime error support type.
public type ProtocolViolation = { protocol: string, message: string };

// Standard runtime error support type.
public type SandboxViolation = { operation: string, target: string };

// Standard runtime error support type.
public type SchemaError = { path: Array<string>, expected: string, actual: string };

// Standard runtime error support type.
public type ToolDenied = { tool: string, reason: string };

// Standard runtime error support type.
public type ToolError = { tool: string, code: string, message: string };

// Standard runtime error support type.
public type ToolTimeout = { tool: string, timeout: std.runtime.time.Duration };

// Standard runtime error support type.
public type ValidationError = { path: Array<string>, message: string };
