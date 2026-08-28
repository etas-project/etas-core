use prost::Message;

#[derive(Clone, PartialEq, Message)]
pub struct ProtoPackageGraphSection {
    #[prost(uint32, tag = "1")]
    pub version: u32,
    #[prost(message, optional, tag = "2")]
    pub package: Option<ProtoPackageIdentity>,
    #[prost(message, repeated, tag = "3")]
    pub dependencies: Vec<ProtoResolvedDependency>,
    #[prost(message, repeated, tag = "4")]
    pub external_modules: Vec<ProtoExternalModule>,
    #[prost(message, optional, tag = "5")]
    pub public_metadata: Option<ProtoPublicMetadataSection>,
    #[prost(message, optional, tag = "6")]
    pub effect_metadata: Option<ProtoEffectMetadataSection>,
    #[prost(message, repeated, tag = "7")]
    pub tool_bindings: Vec<ProtoToolBinding>,
    #[prost(message, repeated, tag = "8")]
    pub bins: Vec<ProtoBinTarget>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoPackageIdentity {
    #[prost(string, tag = "1")]
    pub name: String,
    #[prost(string, tag = "2")]
    pub version: String,
    #[prost(string, tag = "3")]
    pub edition: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoResolvedDependency {
    #[prost(message, optional, tag = "1")]
    pub identity: Option<ProtoPackageIdentity>,
    #[prost(string, tag = "2")]
    pub import_root: String,
    #[prost(message, optional, tag = "3")]
    pub source: Option<ProtoResolvedSource>,
    #[prost(message, repeated, tag = "4")]
    pub dependencies: Vec<ProtoResolvedDependency>,
    #[prost(message, optional, tag = "5")]
    pub public_metadata: Option<ProtoPublicMetadataSection>,
    #[prost(message, optional, tag = "6")]
    pub effect_metadata: Option<ProtoEffectMetadataSection>,
    #[prost(message, repeated, tag = "7")]
    pub tool_bindings: Vec<ProtoToolBinding>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoResolvedSource {
    #[prost(string, tag = "1")]
    pub kind: String,
    #[prost(string, tag = "2")]
    pub registry: String,
    #[prost(string, tag = "3")]
    pub url: String,
    #[prost(string, tag = "4")]
    pub rev: String,
    #[prost(string, tag = "5")]
    pub path: String,
    #[prost(string, tag = "6")]
    pub checksum: String,
    #[prost(string, tag = "7")]
    pub store: String,
    #[prost(string, tag = "8")]
    pub asset_checksum: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoExportsSection {
    #[prost(message, repeated, tag = "1")]
    pub modules: Vec<ProtoExternalModule>,
    #[prost(message, repeated, tag = "2")]
    pub re_exports: Vec<ProtoReExport>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoExternalModule {
    #[prost(message, optional, tag = "1")]
    pub package: Option<ProtoPackageOwner>,
    #[prost(uint32, tag = "2")]
    pub id: u32,
    #[prost(string, repeated, tag = "3")]
    pub path: Vec<String>,
    #[prost(message, repeated, tag = "4")]
    pub exports: Vec<ProtoExternalExport>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoPackageOwner {
    #[prost(message, optional, tag = "1")]
    pub identity: Option<ProtoPackageIdentity>,
    #[prost(string, tag = "2")]
    pub import_root: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoExternalExport {
    #[prost(uint32, tag = "1")]
    pub id: u32,
    #[prost(string, tag = "2")]
    pub name: String,
    #[prost(string, tag = "3")]
    pub visibility: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoPublicMetadataSection {
    #[prost(message, repeated, tag = "1")]
    pub modules: Vec<ProtoExternalModule>,
    #[prost(message, repeated, tag = "2")]
    pub types: Vec<ProtoNamedSignature>,
    #[prost(message, repeated, tag = "3")]
    pub enums: Vec<ProtoNamedSignature>,
    #[prost(message, repeated, tag = "4")]
    pub flows: Vec<ProtoCallableSignature>,
    #[prost(message, repeated, tag = "5")]
    pub agents: Vec<ProtoCallableSignature>,
    #[prost(message, repeated, tag = "6")]
    pub tools: Vec<ProtoCallableSignature>,
    #[prost(message, repeated, tag = "7")]
    pub effects: Vec<ProtoNamedSignature>,
    #[prost(message, repeated, tag = "8")]
    pub actions: Vec<ProtoActionSignature>,
    #[prost(message, repeated, tag = "9")]
    pub trace_specs: Vec<ProtoNamedSignature>,
    #[prost(message, repeated, tag = "10")]
    pub protocols: Vec<ProtoNamedSignature>,
    #[prost(message, repeated, tag = "11")]
    pub effect_summaries: Vec<ProtoEffectSummary>,
    #[prost(message, repeated, tag = "12")]
    pub action_summaries: Vec<ProtoActionSummary>,
    #[prost(message, repeated, tag = "13")]
    pub tool_schemas: Vec<ProtoToolSchema>,
    #[prost(message, repeated, tag = "14")]
    pub trace_spec_summaries: Vec<ProtoTraceSpecSummary>,
    #[prost(message, repeated, tag = "15")]
    pub re_exports: Vec<ProtoReExport>,
    #[prost(string, tag = "16")]
    pub fingerprint: String,
    #[prost(message, repeated, tag = "17")]
    pub values: Vec<ProtoNamedSignature>,
    #[prost(message, repeated, tag = "18")]
    pub annotations: Vec<ProtoAnnotationMetadata>,
    #[prost(message, repeated, tag = "19")]
    pub spec_signatures: Vec<ProtoSpecSignature>,
    #[prost(message, repeated, tag = "20")]
    pub spec_impls: Vec<ProtoSpecImpl>,
    #[prost(message, repeated, tag = "21")]
    pub type_spec_satisfactions: Vec<ProtoTypeSpecSatisfaction>,
    #[prost(message, repeated, tag = "22")]
    pub callable_spec_satisfactions: Vec<ProtoCallableSpecSatisfaction>,
    #[prost(message, repeated, tag = "23")]
    pub trace_spec_conformances: Vec<ProtoTraceSpecConformance>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoAnnotationMetadata {
    #[prost(string, repeated, tag = "1")]
    pub item: Vec<String>,
    #[prost(string, repeated, tag = "2")]
    pub path: Vec<String>,
    #[prost(message, repeated, tag = "3")]
    pub args: Vec<ProtoAnnotationArgMetadata>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoAnnotationArgMetadata {
    #[prost(string, tag = "1")]
    pub name: String,
    #[prost(message, repeated, tag = "2")]
    pub value: Vec<ProtoAnnotationValueMetadata>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoAnnotationFieldMetadata {
    #[prost(string, tag = "1")]
    pub name: String,
    #[prost(message, repeated, tag = "2")]
    pub value: Vec<ProtoAnnotationValueMetadata>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoAnnotationValueMetadata {
    #[prost(string, tag = "1")]
    pub kind: String,
    #[prost(string, tag = "2")]
    pub value: String,
    #[prost(string, repeated, tag = "3")]
    pub path: Vec<String>,
    #[prost(message, repeated, tag = "4")]
    pub elements: Vec<ProtoAnnotationValueMetadata>,
    #[prost(message, repeated, tag = "5")]
    pub fields: Vec<ProtoAnnotationFieldMetadata>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoTypeContractsSection {
    #[prost(message, repeated, tag = "1")]
    pub types: Vec<ProtoNamedSignature>,
    #[prost(message, repeated, tag = "2")]
    pub enums: Vec<ProtoNamedSignature>,
    #[prost(message, repeated, tag = "3")]
    pub flows: Vec<ProtoCallableSignature>,
    #[prost(message, repeated, tag = "4")]
    pub agents: Vec<ProtoCallableSignature>,
    #[prost(message, repeated, tag = "5")]
    pub tools: Vec<ProtoCallableSignature>,
    #[prost(message, repeated, tag = "6")]
    pub effects: Vec<ProtoNamedSignature>,
    #[prost(message, repeated, tag = "7")]
    pub actions: Vec<ProtoActionSignature>,
    #[prost(message, repeated, tag = "8")]
    pub trace_specs: Vec<ProtoNamedSignature>,
    #[prost(message, repeated, tag = "9")]
    pub protocols: Vec<ProtoNamedSignature>,
    #[prost(message, repeated, tag = "10")]
    pub values: Vec<ProtoNamedSignature>,
    #[prost(message, repeated, tag = "11")]
    pub spec_signatures: Vec<ProtoSpecSignature>,
    #[prost(message, repeated, tag = "12")]
    pub spec_impls: Vec<ProtoSpecImpl>,
    #[prost(message, repeated, tag = "13")]
    pub type_spec_satisfactions: Vec<ProtoTypeSpecSatisfaction>,
    #[prost(message, repeated, tag = "14")]
    pub callable_spec_satisfactions: Vec<ProtoCallableSpecSatisfaction>,
    #[prost(message, repeated, tag = "15")]
    pub trace_spec_conformances: Vec<ProtoTraceSpecConformance>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoSpecSignature {
    #[prost(string, repeated, tag = "1")]
    pub path: Vec<String>,
    #[prost(string, tag = "2")]
    pub visibility: String,
    #[prost(string, tag = "3")]
    pub kind: String,
    #[prost(string, repeated, tag = "4")]
    pub param_names: Vec<String>,
    #[prost(message, optional, tag = "5")]
    pub callable: Option<ProtoCallableSignature>,
    #[prost(message, repeated, tag = "6")]
    pub methods: Vec<ProtoSpecMethod>,
    #[prost(message, repeated, tag = "7")]
    pub super_specs: Vec<ProtoSpecBound>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoSpecMethod {
    #[prost(string, tag = "1")]
    pub name: String,
    #[prost(string, repeated, tag = "2")]
    pub path: Vec<String>,
    #[prost(message, optional, tag = "3")]
    pub signature: Option<ProtoCallableSignature>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoSpecBound {
    #[prost(string, repeated, tag = "1")]
    pub spec: Vec<String>,
    #[prost(message, repeated, tag = "2")]
    pub args: Vec<ProtoType>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoSpecImpl {
    #[prost(message, optional, tag = "1")]
    pub self_type: Option<ProtoType>,
    #[prost(string, repeated, tag = "2")]
    pub spec: Vec<String>,
    #[prost(message, repeated, tag = "3")]
    pub args: Vec<ProtoType>,
    #[prost(string, repeated, tag = "4")]
    pub methods: Vec<String>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoTypeSpecSatisfaction {
    #[prost(message, optional, tag = "1")]
    pub self_type: Option<ProtoType>,
    #[prost(string, repeated, tag = "2")]
    pub spec: Vec<String>,
    #[prost(message, repeated, tag = "3")]
    pub args: Vec<ProtoType>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoCallableSpecSatisfaction {
    #[prost(string, repeated, tag = "1")]
    pub item: Vec<String>,
    #[prost(string, repeated, tag = "2")]
    pub spec: Vec<String>,
    #[prost(message, repeated, tag = "3")]
    pub args: Vec<ProtoType>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoTraceSpecConformance {
    #[prost(string, repeated, tag = "1")]
    pub item: Vec<String>,
    #[prost(string, tag = "2")]
    pub target_kind: String,
    #[prost(string, repeated, tag = "3")]
    pub spec: Vec<String>,
    #[prost(message, repeated, tag = "4")]
    pub args: Vec<ProtoType>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoNamedSignature {
    #[prost(string, repeated, tag = "1")]
    pub path: Vec<String>,
    #[prost(string, tag = "2")]
    pub visibility: String,
    #[prost(message, optional, tag = "3")]
    pub ty: Option<ProtoType>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoCallableSignature {
    #[prost(string, repeated, tag = "1")]
    pub path: Vec<String>,
    #[prost(message, repeated, tag = "2")]
    pub input: Vec<ProtoType>,
    #[prost(message, optional, tag = "3")]
    pub output: Option<ProtoType>,
    #[prost(message, optional, tag = "4")]
    pub effects: Option<ProtoEffectRow>,
    #[prost(string, tag = "5")]
    pub visibility: String,
    #[prost(string, repeated, tag = "6")]
    pub param_names: Vec<String>,
    #[prost(message, repeated, tag = "7")]
    pub generic_params: Vec<ProtoActionGenericParam>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoActionSignature {
    #[prost(string, repeated, tag = "1")]
    pub path: Vec<String>,
    #[prost(message, repeated, tag = "2")]
    pub params: Vec<ProtoType>,
    #[prost(message, optional, tag = "3")]
    pub output: Option<ProtoType>,
    #[prost(bool, tag = "4")]
    pub returns_never: bool,
    #[prost(string, tag = "5")]
    pub visibility: String,
    #[prost(message, repeated, tag = "6")]
    pub effect_args: Vec<ProtoActionArgKind>,
    #[prost(string, repeated, tag = "7")]
    pub selector_param_names: Vec<String>,
    #[prost(message, repeated, tag = "8")]
    pub selector_defaults: Vec<ProtoOptionalEffectArg>,
    #[prost(message, repeated, tag = "9")]
    pub generic_params: Vec<ProtoActionGenericParam>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoActionGenericParam {
    #[prost(string, tag = "1")]
    pub name: String,
    #[prost(message, repeated, tag = "2")]
    pub bounds: Vec<ProtoSpecBound>,
    #[prost(string, tag = "3")]
    pub kind: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoActionArgKind {
    #[prost(string, tag = "1")]
    pub kind: String,
    #[prost(string, tag = "2")]
    pub ty: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoOptionalEffectArg {
    #[prost(bool, tag = "1")]
    pub has_value: bool,
    #[prost(message, optional, tag = "2")]
    pub value: Option<ProtoEffectArg>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoType {
    #[prost(string, tag = "1")]
    pub kind: String,
    #[prost(string, tag = "2")]
    pub name: String,
    #[prost(string, repeated, tag = "3")]
    pub path: Vec<String>,
    #[prost(message, repeated, tag = "4")]
    pub children: Vec<ProtoType>,
    #[prost(message, repeated, tag = "5")]
    pub fields: Vec<ProtoTypeField>,
    #[prost(message, optional, tag = "6")]
    pub effects: Option<ProtoEffectRow>,
    #[prost(message, optional, tag = "7")]
    pub produced_effects: Option<ProtoEffectRow>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoTypeField {
    #[prost(string, tag = "1")]
    pub name: String,
    #[prost(message, optional, tag = "2")]
    pub ty: Option<ProtoType>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoEffectRow {
    #[prost(message, repeated, tag = "1")]
    pub effects: Vec<ProtoEffectRef>,
    #[prost(string, optional, tag = "2")]
    pub tail: Option<String>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoEffectRef {
    #[prost(string, repeated, tag = "1")]
    pub path: Vec<String>,
    #[prost(message, repeated, tag = "2")]
    pub args: Vec<ProtoEffectArg>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoEffectArg {
    #[prost(string, tag = "1")]
    pub kind: String,
    #[prost(message, optional, tag = "2")]
    pub ty: Option<ProtoType>,
    #[prost(string, repeated, tag = "3")]
    pub path: Vec<String>,
    #[prost(string, tag = "4")]
    pub value: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoEffectContractsSection {
    #[prost(message, repeated, tag = "1")]
    pub summaries: Vec<ProtoEffectSummary>,
    #[prost(message, repeated, tag = "2")]
    pub tags: Vec<ProtoEffectTag>,
    #[prost(message, repeated, tag = "3")]
    pub extensions: Vec<ProtoEffectExtension>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoEffectMetadataSection {
    #[prost(message, repeated, tag = "1")]
    pub tags: Vec<ProtoEffectTag>,
    #[prost(message, repeated, tag = "2")]
    pub extensions: Vec<ProtoEffectExtension>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoEffectSummary {
    #[prost(string, repeated, tag = "1")]
    pub item: Vec<String>,
    #[prost(message, optional, tag = "2")]
    pub public_effects: Option<ProtoEffectRow>,
    #[prost(message, optional, tag = "3")]
    pub requested_actions: Option<ProtoEffectRow>,
    #[prost(message, repeated, tag = "4")]
    pub latent_flows: Vec<ProtoLatentFlowSummary>,
    #[prost(message, optional, tag = "5")]
    pub handled_requested_actions: Option<ProtoEffectRow>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoLatentFlowSummary {
    #[prost(message, optional, tag = "1")]
    pub declared_bound: Option<ProtoEffectRow>,
    #[prost(message, optional, tag = "2")]
    pub inferred_effects: Option<ProtoEffectRow>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoEffectTag {
    #[prost(string, repeated, tag = "1")]
    pub path: Vec<String>,
    #[prost(string, tag = "2")]
    pub runtime_requirement: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoEffectExtension {
    #[prost(string, repeated, tag = "1")]
    pub child: Vec<String>,
    #[prost(string, repeated, tag = "2")]
    pub parent: Vec<String>,
}

#[allow(dead_code)]
#[derive(Clone, PartialEq, Message)]
pub struct ProtoActionContractsSection {
    #[prost(message, repeated, tag = "1")]
    pub summaries: Vec<ProtoActionSummary>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoActionSummary {
    #[prost(string, repeated, tag = "1")]
    pub action: Vec<String>,
    #[prost(string, repeated, tag = "2")]
    pub args: Vec<String>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoToolContractsSection {
    #[prost(message, repeated, tag = "1")]
    pub signatures: Vec<ProtoCallableSignature>,
    #[prost(message, repeated, tag = "2")]
    pub schemas: Vec<ProtoToolSchema>,
    #[prost(message, repeated, tag = "3")]
    pub bindings: Vec<ProtoToolBinding>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoToolSchema {
    #[prost(string, repeated, tag = "1")]
    pub tool: Vec<String>,
    #[prost(string, tag = "2")]
    pub schema_json: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoToolBinding {
    #[prost(string, tag = "1")]
    pub tool: String,
    #[prost(string, tag = "2")]
    pub kind: String,
    #[prost(string, tag = "3")]
    pub provider: String,
    #[prost(string, repeated, tag = "4")]
    pub effect_row: Vec<String>,
    #[prost(string, repeated, tag = "5")]
    pub action_row: Vec<String>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoTraceSpecContractsSection {
    #[prost(message, repeated, tag = "1")]
    pub trace_specs: Vec<ProtoNamedSignature>,
    #[prost(message, repeated, tag = "2")]
    pub summaries: Vec<ProtoTraceSpecSummary>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoTraceSpecSummary {
    #[prost(string, repeated, tag = "1")]
    pub trace_spec: Vec<String>,
    #[prost(message, repeated, tag = "2")]
    pub clauses: Vec<ProtoTraceSpecClause>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoTraceSpecClause {
    #[prost(string, tag = "1")]
    pub kind: String,
    #[prost(message, optional, tag = "2")]
    pub pattern: Option<ProtoEffectRow>,
    #[prost(message, optional, tag = "3")]
    pub guard: Option<ProtoEffectRow>,
    #[prost(message, optional, tag = "4")]
    pub target: Option<ProtoEffectRow>,
    #[prost(message, optional, tag = "5")]
    pub obligation: Option<ProtoEffectRow>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoReExport {
    #[prost(string, repeated, tag = "1")]
    pub from: Vec<String>,
    #[prost(string, repeated, tag = "2")]
    pub exported: Vec<String>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoPublicSymbolsSection {
    #[prost(message, repeated, tag = "1")]
    pub symbols: Vec<ProtoPublicSymbol>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoPublicSymbol {
    #[prost(string, tag = "1")]
    pub kind: String,
    #[prost(string, repeated, tag = "2")]
    pub path: Vec<String>,
    #[prost(string, tag = "3")]
    pub visibility: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoBinTarget {
    #[prost(string, tag = "1")]
    pub name: String,
    #[prost(string, tag = "2")]
    pub module: String,
    #[prost(string, tag = "3")]
    pub flow: String,
}
