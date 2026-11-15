module std.compiler;

// Ordinary compiler reports are source-owned records. Rust only projects the
// payloads required by the explicit native record ABI.

public type AnalysisStep = {
    callable: std.compiler.CallableId,
    location: Option<std.compiler.DiagnosticLocation>,
    message: string,
};

public type AnnotationArgumentInfo = {
    name: Option<string>,
    value: std.compiler.StaticValueInfo,
};

public type AnnotationInfo = {
    path: Array<string>,
    arguments: Array<std.compiler.AnnotationArgumentInfo>,
};

public type ChangeReport = {
    schema: string,
    identity: std.compiler.DefinitionId,
    old_revision: std.compiler.RevisionInfo,
    new_revision: std.compiler.RevisionInfo,
    old_body_available: bool,
    new_body_available: bool,
    body_changed: Option<bool>,
    signature_changed: bool,
    dependencies_changed: bool,
    declared_bound_changed: bool,
    escaping_changed: Option<bool>,
    requested_changed: Option<bool>,
    default_handled_changed: Option<bool>,
    determinism_changed: Option<bool>,
    trace_changed: Option<bool>,
};

public type CheckReport = {
    status: string,
    diagnostics: Array<std.compiler.DiagnosticRecord>,
    revision: std.compiler.RevisionInfo,
    dependencies: std.compiler.DependencyInfo,
    snapshot_fingerprint: string,
    effects: std.compiler.EffectSummary,
    determinism: Option<std.compiler.Determinism>,
    trace: Option<std.compiler.TraceSummary>,
    runtime_checks: Array<std.compiler.RuntimeCheckInfo>,
};

public type CompileError = {
    kind: string,
    diagnostics: Array<std.compiler.DiagnosticRecord>,
};

public type DefinitionBindingInfo = {
    path: Array<string>,
    definition: std.compiler.DefinitionId,
};

public type DependencyArtifactInfo = {
    import_root: string,
    fingerprint: string,
};

public type DependencyInfo = {
    environment: string,
    lexical_owner: std.compiler.DefinitionId,
    standard_package: string,
    modules: Array<std.compiler.ModuleOriginInfo>,
    bindings: Array<std.compiler.DefinitionBindingInfo>,
    artifacts: Array<std.compiler.DependencyArtifactInfo>,
};

public type DiagnosticDetails = {
    expected: Option<std.compiler.DescriptorDetail>,
    actual: Option<std.compiler.DescriptorDetail>,
    constraint: Option<std.compiler.ViolatedConstraint>,
    related: Array<std.compiler.RelatedLocation>,
    analysis_path: Array<std.compiler.AnalysisStep>,
    trace_witness: Option<std.compiler.TraceWitness>,
};

public type DiagnosticLocation = {
    source: string,
    revision: string,
    byte_start: usize,
    byte_end: usize,
};

public type DiagnosticRecord = {
    code: string,
    severity: string,
    message: string,
    location: Option<std.compiler.DiagnosticLocation>,
    details: Option<std.compiler.DiagnosticDetails>,
};

public type EffectSummary = {
    declared_bound: Option<std.compiler.EffectRowInfo>,
    escaping: Option<std.compiler.EffectRowInfo>,
    requested: Option<std.compiler.ActionSummary>,
    default_handled: Option<std.compiler.ActionSummary>,
};

public type GenericParameterInfo = {
    owner: std.compiler.GenericOwnerInfo,
    position: usize,
    name: string,
    kind: string,
    bounds: Array<std.compiler.SpecInfo>,
};

public type ModuleOriginInfo = {
    module: Array<string>,
    package: string,
};

public type ParameterInfo = {
    name: Option<string>,
    ty: std.compiler.TypeInfo,
};

public type ParseError = {
    diagnostics: Array<std.compiler.DiagnosticRecord>,
};

public type RelatedLocation = {
    location: std.compiler.DiagnosticLocation,
    message: string,
};

public type RevisionInfo = {
    key: string,
    content: string,
    parent: Option<string>,
};

public type RuntimeCheckInfo = {
    kind: string,
    location: Option<std.compiler.DiagnosticLocation>,
    spec: Option<std.compiler.DefinitionId>,
};

public type SignatureInfo = {
    generics: Array<std.compiler.GenericParameterInfo>,
    parameters: Array<std.compiler.ParameterInfo>,
    output: std.compiler.TypeInfo,
    declared_output: Option<std.compiler.TypeInfo>,
    effects: Option<std.compiler.EffectRowInfo>,
};

public type SpecInfo = {
    definition: Option<std.compiler.DefinitionId>,
    arguments: Array<std.compiler.TypeInfo>,
    trace: Option<std.compiler.TraceConstraintInfo>,
};

public type ViolatedConstraint = {
    kind: string,
    owner: Option<std.compiler.DefinitionId>,
    message: string,
};
