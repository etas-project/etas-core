use super::super::super::schema::ProtoDefinitionKey;
use super::super::schema::Callable;

#[derive(Clone, PartialEq, prost::Message)]
pub struct Snapshots {
    #[prost(message, repeated, tag = "1")]
    pub agents: Vec<AgentSnapshot>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct AgentSnapshot {
    #[prost(message, optional, tag = "1")]
    pub identity: Option<ProtoDefinitionKey>,
    #[prost(message, optional, tag = "2")]
    pub revision: Option<Revision>,
    #[prost(message, optional, tag = "3")]
    pub dependencies: Option<Dependencies>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Revision {
    #[prost(message, optional, tag = "1")]
    pub definition: Option<Callable>,
    #[prost(bytes = "vec", tag = "2")]
    pub content: Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub environment: Vec<u8>,
    #[prost(bytes = "vec", tag = "4")]
    pub contract: Vec<u8>,
    #[prost(bytes = "vec", tag = "5")]
    pub compilation: Vec<u8>,
    #[prost(bytes = "vec", optional, tag = "6")]
    pub parent: Option<Vec<u8>>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Dependencies {
    #[prost(bytes = "vec", tag = "1")]
    pub environment: Vec<u8>,
    #[prost(message, optional, tag = "2")]
    pub lexical_owner: Option<ProtoDefinitionKey>,
    #[prost(bytes = "vec", tag = "3")]
    pub standard_package: Vec<u8>,
    #[prost(message, repeated, tag = "4")]
    pub modules: Vec<Module>,
    #[prost(message, repeated, tag = "5")]
    pub bindings: Vec<Binding>,
    #[prost(message, repeated, tag = "6")]
    pub artifacts: Vec<Artifact>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Module {
    #[prost(string, repeated, tag = "1")]
    pub path: Vec<String>,
    #[prost(bytes = "vec", tag = "2")]
    pub package: Vec<u8>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Binding {
    #[prost(string, repeated, tag = "1")]
    pub path: Vec<String>,
    #[prost(message, optional, tag = "2")]
    pub definition: Option<ProtoDefinitionKey>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Artifact {
    #[prost(string, tag = "1")]
    pub import_root: String,
    #[prost(string, tag = "2")]
    pub fingerprint: String,
}
