module std.compiler;

// These fields describe compiler-managed values. The role keeps storage and
// construction inside the compiler; source owns the public projected shape.

@std.role("std.compiler.AgentDef")
public type AgentDef<Actor> = {
    body: Option<std.compiler.SyntaxBlock>,
    effects: std.compiler.EffectSummary,
    determinism: Option<std.compiler.Determinism>,
    trace: Option<std.compiler.TraceSummary>,
    revision: std.compiler.RevisionInfo,
    dependencies: std.compiler.DependencyInfo,
    signature: Option<std.compiler.SignatureInfo>,
    identity: std.compiler.AgentId,
    annotations: Array<std.compiler.AnnotationInfo>,
    tools: Array<std.compiler.ToolRef>,
    specs: Array<std.compiler.SpecInfo>,
    methods: Array<std.compiler.CallableDef>,
};

@std.role("std.compiler.CallableDef")
public type CallableDef<Callable> = {
    body: Option<std.compiler.SyntaxBlock>,
    effects: std.compiler.EffectSummary,
    determinism: Option<std.compiler.Determinism>,
    trace: Option<std.compiler.TraceSummary>,
    revision: std.compiler.RevisionInfo,
    dependencies: std.compiler.DependencyInfo,
    id: std.compiler.CallableId,
    signature: std.compiler.SignatureInfo,
};

@std.role("std.compiler.SyntaxBlock")
public type SyntaxBlock = {
    text: string,
};
