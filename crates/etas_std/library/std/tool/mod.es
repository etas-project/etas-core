module std.tool;

import std.agent.{ToolRef, ToolCall, ToolResult};


// Resolve a checked native tool declaration. This compiler observation does not
// register the tool or authorize a call; a raw name/data value is insufficient.
@std.role("std.tool.reference")
public flow reference<T>(tool: T) -> ToolRef ![];

public flow invoke<C>(call: ToolCall<C>) -> ToolResult<C> ![] {
    let result = perform std.effects.Tool.call<C>(call.tool, call.args, call.id);
    return ToolResult { call_id= call.id, result= result };
}
