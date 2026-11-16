module std.tool;

import std.agent.{ToolRef, ToolCall, ToolResult};

// The checked endpoint requires a resolved native tool declaration. A generic
// data value, flow, or raw name cannot acquire tool identity by this signature.
@std.binding("std.tool.reference")
public flow reference<T>(tool: T) -> ToolRef ![] {}

// The binding retains the conservative behavior of C and checks the registered
// identity/revision, schemas, current authority and inherited execution limits.
@std.binding("std.tool.invoke")
public flow invoke<C>(call: ToolCall<C>) -> ToolResult<C> ![] {}
