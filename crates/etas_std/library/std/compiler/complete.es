module std.compiler;

public flow check<A>(definition: AgentDef<A>) -> CheckReport ![Error<CompileError>] {
    return perform std.effects.Code.check(definition);
}

public flow check<F>(definition: CallableDef<F>) -> CheckReport ![Error<CompileError>] {
    return perform std.effects.Code.check(definition);
}

public flow compile<A>(definition: AgentDef<A>) -> A ![Error<CompileError>] {
    return perform std.effects.Code.compile(definition);
}

public flow compile<F>(definition: CallableDef<F>) -> F ![Error<CompileError>] {
    return perform std.effects.Code.compile(definition);
}

// Replacing a frozen compiler definition is an execution mechanism. Its
// declaration belongs to its actual owner; no module forwarding API exists.
impl AgentDef<A> {
    @std.role("std.compiler.AgentDef.with_body")
    public flow with_body(self, body: SyntaxBlock) -> AgentDef<A> ![];
}

impl CallableDef<F> {
    @std.role("std.compiler.CallableDef.with_body")
    public flow with_body(self, body: SyntaxBlock) -> CallableDef<F> ![];
}
