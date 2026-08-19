#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PackageMetadata {
    pub version: u32,
    pub package: PackageIdentity,
    pub dependencies: Vec<ResolvedDependency>,
    pub external_modules: Vec<ExternalModule>,
    pub public_metadata: PublicMetadata,
    pub effect_metadata: EffectMetadata,
    pub tool_bindings: Vec<ToolBinding>,
    pub bins: Vec<BinTarget>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PackageIdentity {
    pub name: String,
    pub version: String,
    pub edition: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedDependency {
    pub identity: PackageIdentity,
    pub import_root: String,
    pub source: ResolvedDependencySource,
    pub dependencies: Vec<ResolvedDependency>,
    pub public_metadata: PublicMetadata,
    pub effect_metadata: EffectMetadata,
    pub tool_bindings: Vec<ToolBinding>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolvedDependencySource {
    Builtin {
        checksum: String,
    },
    Registry {
        registry: String,
        checksum: String,
        store: Option<String>,
    },
    Git {
        url: String,
        rev: String,
        checksum: String,
        store: Option<String>,
    },
    GitHubClone {
        repo: String,
        rev: String,
        checksum: String,
        store: Option<String>,
    },
    GitHubRelease {
        repo: String,
        release: String,
        asset: String,
        asset_checksum: String,
        payload_checksum: String,
        store: Option<String>,
    },
    Path {
        path: String,
        checksum: String,
    },
    Vendor {
        path: String,
        checksum: String,
        store: Option<String>,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExternalModule {
    pub package: Option<ExternalModuleOwner>,
    pub id: u32,
    pub path: Vec<String>,
    pub exports: Vec<ExternalExport>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExternalModuleOwner {
    pub identity: PackageIdentity,
    pub import_root: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExternalExport {
    pub id: u32,
    pub name: String,
    pub visibility: Visibility,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PublicMetadata {
    pub modules: Vec<ExternalModule>,
    pub types: Vec<NamedSignature>,
    pub values: Vec<NamedSignature>,
    pub enums: Vec<NamedSignature>,
    pub flows: Vec<CallableSignature>,
    pub agents: Vec<CallableSignature>,
    pub tools: Vec<CallableSignature>,
    pub effects: Vec<NamedSignature>,
    pub actions: Vec<ActionSignature>,
    pub trace_specs: Vec<NamedSignature>,
    pub spec_signatures: Vec<SpecSignature>,
    pub spec_impls: Vec<SpecImpl>,
    pub type_spec_satisfactions: Vec<TypeSpecSatisfaction>,
    pub callable_spec_satisfactions: Vec<CallableSpecSatisfaction>,
    pub trace_spec_conformances: Vec<TraceSpecConformance>,
    pub protocols: Vec<NamedSignature>,
    pub effect_summaries: Vec<EffectSummary>,
    pub action_summaries: Vec<ActionSummary>,
    pub tool_schemas: Vec<ToolSchema>,
    pub trace_spec_summaries: Vec<TraceSpecSummary>,
    pub re_exports: Vec<ReExport>,
    pub annotations: Vec<AnnotationMetadata>,
    pub fingerprint: Option<String>,
}

impl PublicMetadata {
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
            && self.types.is_empty()
            && self.values.is_empty()
            && self.enums.is_empty()
            && self.flows.is_empty()
            && self.agents.is_empty()
            && self.tools.is_empty()
            && self.effects.is_empty()
            && self.actions.is_empty()
            && self.trace_specs.is_empty()
            && self.spec_signatures.is_empty()
            && self.spec_impls.is_empty()
            && self.type_spec_satisfactions.is_empty()
            && self.callable_spec_satisfactions.is_empty()
            && self.trace_spec_conformances.is_empty()
            && self.protocols.is_empty()
            && self.effect_summaries.is_empty()
            && self.action_summaries.is_empty()
            && self.tool_schemas.is_empty()
            && self.trace_spec_summaries.is_empty()
            && self.re_exports.is_empty()
            && self.annotations.is_empty()
            && self.fingerprint.is_none()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AnnotationMetadata {
    pub item: Vec<String>,
    pub path: Vec<String>,
    pub args: Vec<AnnotationArgMetadata>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AnnotationArgMetadata {
    pub name: String,
    pub value: AnnotationValueMetadata,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AnnotationFieldMetadata {
    pub name: String,
    pub value: AnnotationValueMetadata,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AnnotationValueMetadata {
    pub kind: AnnotationValueKind,
    pub value: String,
    pub path: Vec<String>,
    pub elements: Vec<AnnotationValueMetadata>,
    pub fields: Vec<AnnotationFieldMetadata>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum AnnotationValueKind {
    #[default]
    Unit,
    Bool,
    Int,
    Float,
    String,
    Char,
    Path,
    Array,
    List,
    Set,
    Tuple,
    Record,
    Constructor,
    Limit,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NamedSignature {
    pub path: Vec<String>,
    pub visibility: Visibility,
    pub ty: Option<Type>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CallableSignature {
    pub path: Vec<String>,
    pub param_names: Vec<String>,
    pub input: Vec<Type>,
    pub output: Option<Type>,
    pub effects: Option<EffectRow>,
    pub visibility: Visibility,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ActionSignature {
    pub path: Vec<String>,
    pub params: Vec<Type>,
    pub effect_args: Vec<ActionArgKind>,
    pub selector_param_names: Vec<String>,
    pub selector_defaults: Vec<Option<EffectArg>>,
    pub output: Option<Type>,
    pub returns_never: bool,
    pub visibility: Visibility,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ActionArgKind {
    #[default]
    Type,
    MemoryPlace,
    StaticResourcePath {
        ty: String,
    },
    StringPattern,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Type {
    pub kind: TypeKind,
    pub name: String,
    pub path: Vec<String>,
    pub children: Vec<Type>,
    pub fields: Vec<TypeField>,
    pub effects: Option<EffectRow>,
    pub produced_effects: Option<EffectRow>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TypeField {
    pub name: String,
    pub ty: Type,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EffectRow {
    pub effects: Vec<EffectRef>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EffectRef {
    pub path: Vec<String>,
    pub args: Vec<EffectArg>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EffectArg {
    pub kind: EffectArgKind,
    pub ty: Option<Type>,
    pub path: Vec<String>,
    pub value: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EffectMetadata {
    pub tags: Vec<EffectTag>,
    pub extensions: Vec<EffectExtension>,
}

impl EffectMetadata {
    pub fn is_empty(&self) -> bool {
        self.tags.is_empty() && self.extensions.is_empty()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EffectSummary {
    pub item: Vec<String>,
    pub public_effects: EffectRow,
    pub requested_actions: EffectRow,
    pub handled_requested_actions: EffectRow,
    pub latent_flows: Vec<LatentFlowSummary>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LatentFlowSummary {
    pub declared_bound: EffectRow,
    pub inferred_effects: EffectRow,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EffectTag {
    pub path: Vec<String>,
    pub runtime_requirement: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EffectExtension {
    pub child: Vec<String>,
    pub parent: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ActionSummary {
    pub action: Vec<String>,
    pub args: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SpecSignature {
    pub path: Vec<String>,
    pub visibility: Visibility,
    pub kind: SpecKind,
    pub param_names: Vec<String>,
    pub callable: Option<CallableSignature>,
    pub methods: Vec<SpecMethod>,
    pub super_specs: Vec<SpecBound>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SpecKind {
    #[default]
    Type,
    Callable,
    Trace,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SpecMethod {
    pub name: String,
    pub path: Vec<String>,
    pub signature: Option<CallableSignature>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SpecBound {
    pub spec: Vec<String>,
    pub args: Vec<Type>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SpecImpl {
    pub self_type: Type,
    pub spec: Vec<String>,
    pub args: Vec<Type>,
    pub methods: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TypeSpecSatisfaction {
    pub self_type: Type,
    pub spec: Vec<String>,
    pub args: Vec<Type>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CallableSpecSatisfaction {
    pub item: Vec<String>,
    pub spec: Vec<String>,
    pub args: Vec<Type>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TraceSpecConformance {
    pub item: Vec<String>,
    pub target: TraceSpecConformanceTarget,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum TraceSpecConformanceTarget {
    #[default]
    Inline,
    Named {
        spec: Vec<String>,
        args: Vec<Type>,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ToolSchema {
    pub tool: Vec<String>,
    pub schema_json: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ToolBinding {
    pub tool: String,
    pub kind: String,
    pub provider: String,
    pub effect_row: Vec<String>,
    pub action_row: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TraceSpecSummary {
    pub trace_spec: Vec<String>,
    pub clauses: Vec<TraceSpecClause>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TraceSpecClause {
    pub kind: TraceSpecClauseKind,
    pub pattern: Option<EffectRow>,
    pub guard: Option<EffectRow>,
    pub target: Option<EffectRow>,
    pub obligation: Option<EffectRow>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum TraceSpecClauseKind {
    #[default]
    Allow,
    Deny,
    RequireBefore,
    RequireAfter,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReExport {
    pub from: Vec<String>,
    pub exported: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PublicSymbols {
    pub symbols: Vec<PublicSymbol>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PublicSymbol {
    pub kind: String,
    pub path: Vec<String>,
    pub visibility: Visibility,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BinTarget {
    pub name: String,
    pub module: String,
    pub flow: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Visibility {
    #[default]
    Public,
    Private,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TypeKind {
    #[default]
    Primitive,
    Var,
    Named,
    Applied,
    Alias,
    Nominal,
    Array,
    List,
    Map,
    Set,
    Range,
    Slice,
    Option,
    Result,
    Record,
    Tuple,
    Function,
    Handler,
    Trusted,
    Untrusted,
    Secret,
    Public,
    Sanitized,
    Prompt,
    PromptPart,
    Message,
    MemorySelection,
    Store,
    MemoryRegion,
    ResourceHandle,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EffectArgKind {
    Type,
    Path,
    String,
    Int,
    #[default]
    Wildcard,
}
