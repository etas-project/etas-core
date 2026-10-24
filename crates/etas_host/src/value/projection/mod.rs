//! Borrowed ABI projections. Sinks choose whether to materialize a value graph
//! or stream its canonical JSON encoding; sources never depend on the sink.

use crate::{HostError, HostJsonValue, HostValue};

mod materialize;
pub use materialize::project_to_host_value;

pub enum HostScalar<'a> {
    Unit,
    Bool(bool),
    Int(i128),
    UInt(u128),
    Float(f64),
    String(&'a str),
    Bytes(&'a [u8]),
}

pub trait HostValueProjection {
    fn project<V: HostValueVisitor>(&self, visitor: V) -> Result<V::Output, HostError>;
}

pub trait HostValueVisitor: Sized {
    type Output;
    fn scalar(self, value: HostScalar<'_>) -> Result<Self::Output, HostError>;
    fn list<P: HostValueProjection>(
        self,
        values: impl IntoIterator<Item = P>,
    ) -> Result<Self::Output, HostError>;
    fn map<P: HostValueProjection>(
        self,
        entries: impl IntoIterator<Item = (P, P)>,
    ) -> Result<Self::Output, HostError>;
    fn record<'a, P: HostValueProjection>(
        self,
        fields: impl IntoIterator<Item = (&'a str, P)>,
    ) -> Result<Self::Output, HostError>;
    fn variant<P: HostValueProjection>(
        self,
        name: &str,
        fields: impl IntoIterator<Item = P>,
    ) -> Result<Self::Output, HostError>;
    fn json<P: HostJsonProjection>(self, value: &P) -> Result<Self::Output, HostError>;
}

pub trait HostJsonProjection {
    fn project_json<V: HostJsonVisitor>(&self, visitor: V) -> Result<V::Output, HostError>;
}

pub trait HostJsonVisitor: Sized {
    type Output;
    fn null(self) -> Result<Self::Output, HostError>;
    fn boolean(self, value: bool) -> Result<Self::Output, HostError>;
    fn number(self, value: f64) -> Result<Self::Output, HostError>;
    fn string(self, value: &str) -> Result<Self::Output, HostError>;
    fn array<P: HostJsonProjection>(
        self,
        values: impl IntoIterator<Item = P>,
    ) -> Result<Self::Output, HostError>;
    fn object<'a, P: HostJsonProjection>(
        self,
        fields: impl IntoIterator<Item = (&'a str, P)>,
    ) -> Result<Self::Output, HostError>;
}

impl<P: HostValueProjection + ?Sized> HostValueProjection for &P {
    fn project<V: HostValueVisitor>(&self, visitor: V) -> Result<V::Output, HostError> {
        (*self).project(visitor)
    }
}

impl<P: HostJsonProjection + ?Sized> HostJsonProjection for &P {
    fn project_json<V: HostJsonVisitor>(&self, visitor: V) -> Result<V::Output, HostError> {
        (*self).project_json(visitor)
    }
}

impl HostValueProjection for HostValue {
    fn project<V: HostValueVisitor>(&self, visitor: V) -> Result<V::Output, HostError> {
        match self {
            Self::Unit => visitor.scalar(HostScalar::Unit),
            Self::Bool(v) => visitor.scalar(HostScalar::Bool(*v)),
            Self::Int(v) => visitor.scalar(HostScalar::Int(*v)),
            Self::UInt(v) => visitor.scalar(HostScalar::UInt(*v)),
            Self::Float(v) => visitor.scalar(HostScalar::Float(*v)),
            Self::String(v) => visitor.scalar(HostScalar::String(v)),
            Self::Bytes(v) => visitor.scalar(HostScalar::Bytes(v)),
            Self::List(v) => visitor.list(v),
            Self::Map(v) => visitor.map(v.iter().map(|(k, v)| (k, v))),
            Self::Record(v) => visitor.record(v.iter().map(|(k, v)| (k.as_str(), v))),
            Self::Variant { name, fields } => visitor.variant(name, fields),
            Self::Json(v) => visitor.json(v),
        }
    }
}

impl HostJsonProjection for HostJsonValue {
    fn project_json<V: HostJsonVisitor>(&self, visitor: V) -> Result<V::Output, HostError> {
        match self {
            Self::Null => visitor.null(),
            Self::Bool(v) => visitor.boolean(*v),
            Self::Number(v) => visitor.number(*v),
            Self::String(v) => visitor.string(v),
            Self::Array(v) => visitor.array(v),
            Self::Object(v) => visitor.object(v.iter().map(|(k, v)| (k.as_str(), v))),
        }
    }
}
