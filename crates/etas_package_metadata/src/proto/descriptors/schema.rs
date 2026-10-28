use super::super::schema::ProtoDefinitionKey;

#[derive(Clone, PartialEq, prost::Message)]
pub struct Graph {
    #[prost(uint32, tag = "1")]
    pub version: u32,
    #[prost(message, repeated, tag = "2")]
    pub types: Vec<Type>,
    #[prost(message, repeated, tag = "3")]
    pub rows: Vec<Row>,
    #[prost(message, repeated, tag = "4")]
    pub binders: Vec<Binder>,
    #[prost(message, repeated, tag = "5")]
    pub traces: Vec<Trace>,
    #[prost(message, repeated, tag = "6")]
    pub declarations: Vec<Binding>,
    #[prost(message, optional, tag = "7")]
    pub headers: Option<Headers>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Headers {
    #[prost(message, repeated, tag = "1")]
    pub callables: Vec<CallableHeader>,
    #[prost(message, repeated, tag = "2")]
    pub agents: Vec<AgentHeader>,
    #[prost(message, optional, tag = "3")]
    pub snapshots: Option<super::snapshots::schema::Snapshots>,
    #[prost(message, optional, tag = "4")]
    pub bodies: Option<super::bodies::Bodies>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct CallableHeader {
    #[prost(message, optional, tag = "1")]
    pub id: Option<Callable>,
    #[prost(uint32, tag = "2")]
    pub signature: u32,
    #[prost(message, optional, tag = "3")]
    pub analysis: Option<Analysis>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Analysis {
    #[prost(uint32, optional, tag = "1")]
    pub declared_bound: Option<u32>,
    #[prost(uint32, optional, tag = "2")]
    pub escaping: Option<u32>,
    #[prost(uint32, optional, tag = "3")]
    pub requested: Option<u32>,
    #[prost(uint32, optional, tag = "4")]
    pub default_handled: Option<u32>,
    #[prost(uint32, optional, tag = "5")]
    pub determinism: Option<u32>,
    #[prost(uint32, optional, tag = "6")]
    pub trace: Option<u32>,
    #[prost(uint32, optional, tag = "7")]
    pub outcomes: Option<u32>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct AgentHeader {
    #[prost(message, optional, tag = "1")]
    pub identity: Option<ProtoDefinitionKey>,
    #[prost(uint32, tag = "2")]
    pub component: u32,
    #[prost(message, optional, tag = "3")]
    pub default_entry: Option<Callable>,
    #[prost(message, repeated, tag = "4")]
    pub methods: Vec<Callable>,
    #[prost(message, repeated, tag = "5")]
    pub tools: Vec<ProtoDefinitionKey>,
    #[prost(message, repeated, tag = "6")]
    pub annotations: Vec<Annotation>,
    #[prost(message, repeated, tag = "7")]
    pub specs: Vec<AttachedSpec>,
    #[prost(message, optional, tag = "8")]
    pub method_info: Option<AgentMethods>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct AgentMethods {
    #[prost(message, repeated, tag = "1")]
    pub methods: Vec<AgentMethod>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct AgentMethod {
    #[prost(message, optional, tag = "1")]
    pub callable: Option<Callable>,
    #[prost(bool, tag = "2")]
    pub inherent: bool,
    #[prost(message, repeated, tag = "3")]
    pub specs: Vec<Spec>,
    #[prost(bool, optional, tag = "4")]
    pub is_public: Option<bool>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct AnnotationReference {
    #[prost(oneof = "annotation_reference::Kind", tags = "1, 2")]
    pub kind: Option<annotation_reference::Kind>,
}
pub mod annotation_reference {
    #[derive(Clone, PartialEq, prost::Oneof)]
    pub enum Kind {
        #[prost(message, tag = "1")]
        Declaration(super::ProtoDefinitionKey),
        #[prost(message, tag = "2")]
        Module(super::ModuleReference),
    }
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct ModuleReference {
    #[prost(bytes = "vec", tag = "1")]
    pub package: Vec<u8>,
    #[prost(string, repeated, tag = "2")]
    pub path: Vec<String>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct StaticArgument {
    #[prost(string, optional, tag = "1")]
    pub name: Option<String>,
    #[prost(uint32, tag = "2")]
    pub value: u32,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Annotation {
    #[prost(string, repeated, tag = "1")]
    pub path: Vec<String>,
    #[prost(message, repeated, tag = "2")]
    pub arguments: Vec<StaticArgument>,
    #[prost(message, repeated, tag = "3")]
    pub values: Vec<StaticValue>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct StaticValue {
    #[prost(oneof = "static_value::Kind", tags = "1, 2, 3, 4, 5, 6, 7, 8, 9, 10")]
    pub kind: Option<static_value::Kind>,
}
pub mod static_value {
    #[derive(Clone, PartialEq, prost::Oneof)]
    pub enum Kind {
        #[prost(bool, tag = "1")]
        Unit(bool),
        #[prost(bool, tag = "2")]
        Bool(bool),
        #[prost(string, tag = "3")]
        Integer(String),
        #[prost(string, tag = "4")]
        Float(String),
        #[prost(string, tag = "5")]
        String(String),
        #[prost(uint32, tag = "6")]
        Character(u32),
        #[prost(message, tag = "7")]
        Reference(super::AnnotationReference),
        #[prost(message, tag = "8")]
        Sequence(super::StaticSequence),
        #[prost(message, tag = "9")]
        Record(super::StaticRecord),
        #[prost(message, tag = "10")]
        Constructor(super::StaticConstructor),
    }
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct StaticSequence {
    #[prost(uint32, tag = "1")]
    pub kind: u32,
    #[prost(uint32, repeated, tag = "2")]
    pub elements: Vec<u32>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct StaticRecord {
    #[prost(message, repeated, tag = "1")]
    pub fields: Vec<StaticField>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct StaticField {
    #[prost(string, tag = "1")]
    pub name: String,
    #[prost(uint32, tag = "2")]
    pub value: u32,
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct StaticConstructor {
    #[prost(uint32, tag = "1")]
    pub kind: u32,
    #[prost(message, optional, tag = "2")]
    pub target: Option<AnnotationReference>,
    #[prost(message, repeated, tag = "3")]
    pub arguments: Vec<StaticArgument>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct AttachedSpec {
    #[prost(oneof = "attached_spec::Kind", tags = "1, 2, 3")]
    pub kind: Option<attached_spec::Kind>,
}
pub mod attached_spec {
    #[derive(Clone, PartialEq, prost::Oneof)]
    pub enum Kind {
        #[prost(message, tag = "1")]
        Named(super::Spec),
        #[prost(message, tag = "2")]
        Inline(super::TraceConstraint),
        #[prost(message, tag = "3")]
        Protocol(super::Spec),
    }
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct TraceConstraint {
    #[prost(message, repeated, tag = "1")]
    pub alternatives: Vec<TraceRules>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct TraceRules {
    #[prost(message, repeated, tag = "1")]
    pub rules: Vec<TraceRule>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct TraceRule {
    #[prost(uint32, tag = "1")]
    pub kind: u32,
    #[prost(message, repeated, tag = "2")]
    pub patterns: Vec<TracePattern>,
    #[prost(uint64, optional, tag = "3")]
    pub count: Option<u64>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct TracePattern {
    #[prost(uint32, tag = "1")]
    pub row: u32,
    #[prost(message, repeated, tag = "2")]
    pub bounds: Vec<SelectorBound>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct SelectorBound {
    #[prost(message, optional, tag = "1")]
    pub target: Option<EffectTarget>,
    #[prost(uint32, tag = "2")]
    pub argument: u32,
    #[prost(message, optional, tag = "3")]
    pub spec: Option<Spec>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Binding {
    #[prost(message, optional, tag = "1")]
    pub definition: Option<ProtoDefinitionKey>,
    #[prost(uint32, tag = "2")]
    pub ty: u32,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Refs {
    #[prost(uint32, repeated, tag = "1")]
    pub values: Vec<u32>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Field {
    #[prost(string, tag = "1")]
    pub name: String,
    #[prost(uint32, tag = "2")]
    pub ty: u32,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Fields {
    #[prost(message, repeated, tag = "1")]
    pub values: Vec<Field>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Variant {
    #[prost(string, tag = "1")]
    pub name: String,
    #[prost(oneof = "variant::Payload", tags = "2, 3, 4")]
    pub payload: Option<variant::Payload>,
}
pub mod variant {
    #[derive(Clone, PartialEq, prost::Oneof)]
    pub enum Payload {
        #[prost(bool, tag = "2")]
        None(bool),
        #[prost(message, tag = "3")]
        Positional(super::Refs),
        #[prost(message, tag = "4")]
        Named(super::Fields),
    }
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Builtin {
    #[prost(uint32, tag = "1")]
    pub constructor: u32,
    #[prost(uint32, repeated, tag = "2")]
    pub arguments: Vec<u32>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Declaration {
    #[prost(message, optional, tag = "1")]
    pub definition: Option<ProtoDefinitionKey>,
    #[prost(uint32, repeated, tag = "2")]
    pub parameters: Vec<u32>,
    #[prost(uint32, optional, tag = "3")]
    pub representation: Option<u32>,
    #[prost(message, repeated, tag = "4")]
    pub variants: Vec<Variant>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Applied {
    #[prost(uint32, tag = "1")]
    pub constructor: u32,
    #[prost(uint32, repeated, tag = "2")]
    pub arguments: Vec<u32>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Parameter {
    #[prost(string, optional, tag = "1")]
    pub name: Option<String>,
    #[prost(uint32, tag = "2")]
    pub ty: u32,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Signature {
    #[prost(uint32, repeated, tag = "1")]
    pub generics: Vec<u32>,
    #[prost(message, repeated, tag = "2")]
    pub parameters: Vec<Parameter>,
    #[prost(uint32, tag = "3")]
    pub output: u32,
    #[prost(uint32, optional, tag = "4")]
    pub declared_output: Option<u32>,
    #[prost(uint32, optional, tag = "5")]
    pub effects: Option<u32>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Handler {
    #[prost(uint32, tag = "1")]
    pub handled: u32,
    #[prost(uint32, optional, tag = "2")]
    pub produced: Option<u32>,
    #[prost(uint32, optional, tag = "3")]
    pub result: Option<u32>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Type {
    #[prost(oneof = "ty::Kind", tags = "1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11")]
    pub kind: Option<ty::Kind>,
}
pub mod ty {
    #[derive(Clone, PartialEq, prost::Oneof)]
    pub enum Kind {
        #[prost(uint32, tag = "1")]
        Primitive(u32),
        #[prost(message, tag = "2")]
        Builtin(super::Builtin),
        #[prost(uint32, tag = "3")]
        Parameter(u32),
        #[prost(message, tag = "4")]
        Tuple(super::Refs),
        #[prost(message, tag = "5")]
        Record(super::Fields),
        #[prost(message, tag = "6")]
        Nominal(super::Declaration),
        #[prost(message, tag = "7")]
        Alias(super::Declaration),
        #[prost(message, tag = "8")]
        Enum(super::Declaration),
        #[prost(message, tag = "9")]
        Applied(super::Applied),
        #[prost(message, tag = "10")]
        Function(super::Signature),
        #[prost(message, tag = "11")]
        Handler(super::Handler),
    }
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct EffectTarget {
    #[prost(message, optional, tag = "1")]
    pub effect: Option<ProtoDefinitionKey>,
    #[prost(string, optional, tag = "2")]
    pub action: Option<String>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct StaticArg {
    #[prost(oneof = "static_arg::Kind", tags = "1, 2, 3, 4, 5, 6")]
    pub kind: Option<static_arg::Kind>,
}
pub mod static_arg {
    #[derive(Clone, PartialEq, prost::Oneof)]
    pub enum Kind {
        #[prost(uint32, tag = "1")]
        Type(u32),
        #[prost(message, tag = "2")]
        Resource(super::ProtoDefinitionKey),
        #[prost(string, tag = "3")]
        String(String),
        #[prost(string, tag = "4")]
        Integer(String),
        #[prost(bool, tag = "5")]
        Wildcard(bool),
        #[prost(message, tag = "6")]
        Callable(super::Callable),
    }
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Effect {
    #[prost(message, optional, tag = "1")]
    pub target: Option<EffectTarget>,
    #[prost(message, repeated, tag = "2")]
    pub arguments: Vec<StaticArg>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Row {
    #[prost(message, repeated, tag = "1")]
    pub effects: Vec<Effect>,
    #[prost(uint32, optional, tag = "2")]
    pub tail: Option<u32>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Method {
    #[prost(bytes = "vec", tag = "1")]
    pub package: Vec<u8>,
    #[prost(string, repeated, tag = "2")]
    pub module: Vec<String>,
    #[prost(message, optional, tag = "3")]
    pub subject: Option<ProtoDefinitionKey>,
    #[prost(message, optional, tag = "4")]
    pub selected_spec: Option<ProtoDefinitionKey>,
    #[prost(bytes = "vec", tag = "5")]
    pub header: Vec<u8>,
    #[prost(string, tag = "6")]
    pub name: String,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Callable {
    #[prost(oneof = "callable::Kind", tags = "1, 2")]
    pub kind: Option<callable::Kind>,
}
pub mod callable {
    #[derive(Clone, PartialEq, prost::Oneof)]
    pub enum Kind {
        #[prost(message, tag = "1")]
        Declaration(super::ProtoDefinitionKey),
        #[prost(message, tag = "2")]
        Method(super::Method),
    }
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Spec {
    #[prost(message, optional, tag = "1")]
    pub definition: Option<ProtoDefinitionKey>,
    #[prost(uint32, repeated, tag = "2")]
    pub arguments: Vec<u32>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Binder {
    #[prost(message, optional, tag = "1")]
    pub owner: Option<Callable>,
    #[prost(uint32, tag = "2")]
    pub position: u32,
    #[prost(string, tag = "3")]
    pub name: String,
    #[prost(uint32, tag = "4")]
    pub kind: u32,
    #[prost(message, repeated, tag = "5")]
    pub bounds: Vec<Spec>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct ParameterCall {
    #[prost(message, optional, tag = "1")]
    pub callable: Option<Callable>,
    #[prost(uint32, tag = "2")]
    pub signature: u32,
    #[prost(uint32, tag = "3")]
    pub position: u32,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Observation {
    #[prost(oneof = "observation::Kind", tags = "1, 2, 3, 4")]
    pub kind: Option<observation::Kind>,
}
pub mod observation {
    #[derive(Clone, PartialEq, prost::Oneof)]
    pub enum Kind {
        #[prost(bool, tag = "1")]
        Any(bool),
        #[prost(bool, tag = "2")]
        Unit(bool),
        #[prost(bool, tag = "3")]
        Bool(bool),
        #[prost(message, tag = "4")]
        Variant(super::VariantObservation),
    }
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct VariantObservation {
    #[prost(message, optional, tag = "1")]
    pub owner: Option<ProtoDefinitionKey>,
    #[prost(string, tag = "2")]
    pub name: String,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Returned {
    #[prost(uint32, tag = "1")]
    pub ty: u32,
    #[prost(message, optional, tag = "2")]
    pub observation: Option<Observation>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Outcome {
    #[prost(oneof = "outcome::Kind", tags = "1, 2, 3")]
    pub kind: Option<outcome::Kind>,
}
pub mod outcome {
    #[derive(Clone, PartialEq, prost::Oneof)]
    pub enum Kind {
        #[prost(message, tag = "1")]
        Returned(super::Returned),
        #[prost(uint32, tag = "2")]
        Raised(u32),
        #[prost(bool, tag = "3")]
        NonReturn(bool),
    }
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Widened {
    #[prost(uint32, tag = "1")]
    pub actions: u32,
    #[prost(message, repeated, tag = "2")]
    pub parameter_calls: Vec<ParameterCall>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Trace {
    #[prost(oneof = "trace::Kind", tags = "1, 2, 3, 4, 5, 6, 7, 8, 9")]
    pub kind: Option<trace::Kind>,
}
pub mod trace {
    #[derive(Clone, PartialEq, prost::Oneof)]
    pub enum Kind {
        #[prost(bool, tag = "1")]
        Empty(bool),
        #[prost(message, tag = "2")]
        Event(super::Effect),
        #[prost(message, tag = "3")]
        Outcome(super::Outcome),
        #[prost(message, tag = "4")]
        ParameterCall(super::ParameterCall),
        #[prost(message, tag = "5")]
        Seq(super::Refs),
        #[prost(message, tag = "6")]
        Choice(super::Refs),
        #[prost(uint32, tag = "7")]
        Repeat(u32),
        #[prost(uint32, tag = "8")]
        UnknownOrder(u32),
        #[prost(message, tag = "9")]
        Widened(super::Widened),
    }
}
