module std.compiler;

// Source data describes each definition; the retained handle carries the live
// compiler observation. Descriptive IDs alone cannot authorize compilation.
type RetainedDefinition;
type RetainedSyntax;

public type AgentDef<Actor> = {
    private retained: RetainedDefinition,
    private observation: DefinitionObservation,
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

public type CallableDef<Callable> = {
    private retained: RetainedDefinition,
    private observation: DefinitionObservation,
    body: Option<std.compiler.SyntaxBlock>,
    effects: std.compiler.EffectSummary,
    determinism: Option<std.compiler.Determinism>,
    trace: Option<std.compiler.TraceSummary>,
    revision: std.compiler.RevisionInfo,
    dependencies: std.compiler.DependencyInfo,
    id: std.compiler.CallableId,
    signature: std.compiler.SignatureInfo,
};

public type SyntaxBlock = {
    private retained: RetainedSyntax,
    // Canonical compiler observation. The public format flow applies Etas
    // formatting policy to the token observations below.
    text: string,
    private tokens: Array<SyntaxToken>,
    private format_limit: usize,
};

type DefinitionObservation = {
    identity: DefinitionId,
    entry: string,
    environment: string,
    signature: Option<string>,
    body: Option<string>,
    declared_bound: Option<string>,
    escaping: Option<string>,
    requested: Option<string>,
    default_handled: Option<string>,
    determinism: Option<Determinism>,
    trace: Option<string>,
};

// Readonly lexer observations tied to the retained source and token spans.
// Whitespace/comments remain in the observations so source owns their omission.
enum SyntaxTokenKind {
    Identifier,
    Keyword,
    Punctuation,
    Other,
    Trivia,
    Eof,
}

type SyntaxToken = { kind: SyntaxTokenKind, text: string };
