use super::{
    ProtoActionGenericParam, ProtoActionTrace, ProtoDefaultHandlerEffects, ProtoEffectRow,
    ProtoRequestInstantiationTemplate, ProtoSpecBound, ProtoType,
};
use prost::Message;

#[derive(Clone, PartialEq, Message)]
pub struct ProtoCallableValueGraph {
    #[prost(message, repeated, tag = "1")]
    pub sources: Vec<ProtoCallableValueSource>,
    #[prost(message, repeated, tag = "2")]
    pub values: Vec<ProtoCallableValueNode>,
    #[prost(message, repeated, tag = "3")]
    pub objects: Vec<ProtoExistentialValueNode>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoExistentialValueNode {
    #[prost(message, optional, tag = "1")]
    pub parent_scope: Option<ProtoCallableValueScope>,
    #[prost(message, repeated, tag = "2")]
    pub generic_bindings: Vec<ProtoCallableValueGenericBinding>,
    #[prost(message, optional, tag = "3")]
    pub object_type: Option<ProtoType>,
    #[prost(message, optional, tag = "4")]
    pub source_type: Option<ProtoType>,
    #[prost(message, repeated, tag = "5")]
    pub payload_sources: Vec<ProtoCallableValueSource>,
    #[prost(message, repeated, tag = "6")]
    pub witnesses: Vec<ProtoExistentialMemberWitness>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoExistentialMemberWitness {
    #[prost(message, optional, tag = "1")]
    pub bound: Option<ProtoSpecBound>,
    #[prost(string, tag = "2")]
    pub member: String,
    #[prost(oneof = "object_witness::Target", tags = "3, 4")]
    pub target: Option<object_witness::Target>,
}

pub mod object_witness {
    #[derive(Clone, PartialEq, prost::Oneof)]
    pub enum Target {
        #[prost(message, tag = "3")]
        Implementation(Box<super::ProtoExistentialMethodImplementation>),
        #[prost(bool, tag = "4")]
        Bound(bool),
    }
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoExistentialMethodImplementation {
    #[prost(message, optional, tag = "1")]
    pub method: Option<super::super::descriptors::ProtoMethodIdentity>,
    #[prost(uint32, tag = "2")]
    pub value: u32,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoCallableValueScope {
    #[prost(oneof = "scope::Kind", tags = "1, 2")]
    pub kind: Option<scope::Kind>,
}

pub mod scope {
    #[derive(Clone, PartialEq, prost::Oneof)]
    pub enum Kind {
        #[prost(bool, tag = "1")]
        Root(bool),
        #[prost(uint32, tag = "2")]
        Value(u32),
    }
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoCallableValueParameter {
    #[prost(message, optional, tag = "1")]
    pub scope: Option<ProtoCallableValueScope>,
    #[prost(uint32, tag = "2")]
    pub position: u32,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoCallableValueSource {
    #[prost(oneof = "source::Kind", tags = "1, 2, 3, 4")]
    pub kind: Option<source::Kind>,
}

pub mod source {
    #[derive(Clone, PartialEq, prost::Oneof)]
    pub enum Kind {
        #[prost(message, tag = "1")]
        Parameter(super::ProtoCallableValueParameter),
        #[prost(uint32, tag = "2")]
        Value(u32),
        #[prost(uint32, tag = "3")]
        FrozenContract(u32),
        #[prost(uint32, tag = "4")]
        Existential(u32),
    }
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoCallableValueSources {
    #[prost(message, repeated, tag = "1")]
    pub sources: Vec<ProtoCallableValueSource>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoCallableValueParameterBinding {
    #[prost(string, tag = "1")]
    pub label: String,
    #[prost(message, optional, tag = "2")]
    pub parameter: Option<ProtoCallableValueParameter>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoCallableValueGenericBinding {
    #[prost(string, tag = "1")]
    pub label: String,
    #[prost(message, optional, tag = "2")]
    pub scope: Option<ProtoCallableValueScope>,
    #[prost(uint32, tag = "3")]
    pub position: u32,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoCallableValueNode {
    #[prost(message, optional, tag = "16")]
    pub residual_checks: Option<ProtoResidualReferenceSet>,
    #[prost(message, optional, tag = "15")]
    pub handler_requests: Option<ProtoRequestInstantiationTemplate>,
    #[prost(message, optional, tag = "14")]
    pub behavior: Option<ProtoCallableValueBehavior>,
    #[prost(message, optional, tag = "13")]
    pub parent_scope: Option<ProtoCallableValueScope>,
    #[prost(message, repeated, tag = "1")]
    pub generic_params: Vec<ProtoActionGenericParam>,
    #[prost(message, repeated, tag = "2")]
    pub generic_bindings: Vec<ProtoCallableValueGenericBinding>,
    #[prost(string, repeated, tag = "3")]
    pub param_names: Vec<String>,
    #[prost(message, repeated, tag = "4")]
    pub params: Vec<ProtoType>,
    #[prost(message, optional, tag = "5")]
    pub output: Option<ProtoType>,
    #[prost(message, optional, tag = "6")]
    pub public_effects: Option<ProtoEffectRow>,
    #[prost(message, optional, tag = "7")]
    pub requested_actions: Option<ProtoEffectRow>,
    #[prost(message, optional, tag = "8")]
    pub handled_requested_actions: Option<ProtoEffectRow>,
    #[prost(message, optional, tag = "9")]
    pub default_handler_effects: Option<ProtoDefaultHandlerEffects>,
    #[prost(message, repeated, tag = "10")]
    pub parameter_bindings: Vec<ProtoCallableValueParameterBinding>,
    #[prost(message, optional, tag = "11")]
    pub action_trace: Option<ProtoActionTrace>,
    #[prost(message, optional, tag = "12")]
    pub returned_sources: Option<ProtoCallableValueSources>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoResidualReferenceSet {
    #[prost(uint32, tag = "1")]
    pub version: u32,
    #[prost(message, repeated, tag = "2")]
    pub references: Vec<ProtoResidualReference>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoResidualReference {
    #[prost(bytes = "vec", tag = "1")]
    pub obligation: Vec<u8>,
    #[prost(message, repeated, tag = "2")]
    pub substitutions: Vec<ProtoResidualSubstitution>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoResidualSubstitution {
    #[prost(bytes = "vec", tag = "1")]
    pub parameter: Vec<u8>,
    #[prost(uint32, tag = "2")]
    pub kind: u32,
    #[prost(bytes = "vec", tag = "3")]
    pub value: Vec<u8>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoCallableValueBehavior {
    #[prost(int32, optional, tag = "1")]
    pub determinism: Option<i32>,
    #[prost(int32, repeated, tag = "2")]
    pub host_requirements: Vec<i32>,
    #[prost(int32, repeated, tag = "3")]
    pub interpreter_features: Vec<i32>,
    #[prost(int32, optional, tag = "4")]
    pub independent_determinism: Option<i32>,
}
