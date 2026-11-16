module std.compiler;

// Public declarations migrated from the legacy API inventory.
// Maintained as Etas source; native bindings retain existing execution during migration.

// Check the immutable definition in its frozen environment without executing it.
@std.binding("std.compiler.check")
public flow check<A>(arg0: std.compiler.AgentDef<A>) -> std.compiler.CheckReport ![Error<std.compiler.CompileError>] {}

// Alternate public signature for the same native operation.
@std.binding("std.compiler.check")
public flow check<F>(arg0: std.compiler.CallableDef<F>) -> std.compiler.CheckReport ![Error<std.compiler.CompileError>] {}

// Check the immutable definition in its frozen environment without executing it.
@std.binding("std.compiler.compile")
public flow compile<A>(arg0: std.compiler.AgentDef<A>) -> A ![Error<std.compiler.CompileError>] {}

// Alternate public signature for the same native operation.
@std.binding("std.compiler.compile")
public flow compile<F>(arg0: std.compiler.CallableDef<F>) -> F ![Error<std.compiler.CompileError>] {}

// Compare available immutable definition data, not behavioral compatibility.
@std.binding("std.compiler.diff")
public flow diff<A>(arg0: std.compiler.AgentDef<A>, arg1: std.compiler.AgentDef<A>) -> std.compiler.ChangeReport ![] {}

// Alternate public signature for the same native operation.
@std.binding("std.compiler.diff")
public flow diff<F>(arg0: std.compiler.CallableDef<F>, arg1: std.compiler.CallableDef<F>) -> std.compiler.ChangeReport ![] {}

// Replace immutable definition body syntax without checking or executing it.
@std.binding("std.compiler.with_body")
public flow with_body<A>(arg0: std.compiler.AgentDef<A>, arg1: std.compiler.SyntaxBlock) -> std.compiler.AgentDef<A> ![] {}

// Alternate public signature for the same native operation.
@std.binding("std.compiler.with_body")
public flow with_body<F>(arg0: std.compiler.CallableDef<F>, arg1: std.compiler.SyntaxBlock) -> std.compiler.CallableDef<F> ![] {}

impl std.compiler.AgentDef<A> {
    public flow with_body(self, arg0: std.compiler.SyntaxBlock) -> std.compiler.AgentDef<A> ![] { return std.compiler.with_body(self, arg0); }
}
