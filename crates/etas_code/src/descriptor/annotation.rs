use std::sync::Arc;

use crate::{CodeFingerprint, DefinitionKey, PackageKey, identity::canonical::CanonicalEncoder};

/// Static annotation descriptions, never execution authority or spec evidence.
#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub struct AnnotationInfo<R = AnnotationReferenceInfo> {
    pub path: Vec<String>,
    pub arguments: Vec<AnnotationArgumentInfo<R>>,
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub struct AnnotationArgumentInfo<R = AnnotationReferenceInfo> {
    pub name: Option<String>,
    pub value: Arc<StaticValueInfo<R>>,
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub enum AnnotationReferenceInfo {
    Declaration(DefinitionKey),
    Module {
        package: PackageKey,
        path: Vec<String>,
    },
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub enum AnnotationSequenceKind {
    Array,
    List,
    Set,
    Tuple,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub enum AnnotationConstructorKind {
    Limit,
    Trace,
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub enum AnnotationLiteralInfo {
    Unit,
    Bool(bool),
    Integer(String),
    Float(String),
    String(String),
    Character(char),
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub enum StaticValueInfo<R = AnnotationReferenceInfo> {
    Literal(AnnotationLiteralInfo),
    Reference(R),
    Sequence {
        kind: AnnotationSequenceKind,
        elements: Vec<Arc<StaticValueInfo<R>>>,
    },
    Record(Vec<AnnotationFieldInfo<R>>),
    Constructor {
        kind: AnnotationConstructorKind,
        target: R,
        arguments: Vec<AnnotationArgumentInfo<R>>,
    },
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub struct AnnotationFieldInfo<R = AnnotationReferenceInfo> {
    pub name: String,
    pub value: Arc<StaticValueInfo<R>>,
}

impl AnnotationInfo {
    pub fn fingerprint(&self) -> CodeFingerprint {
        let mut encoder = CanonicalEncoder::new("etas.code.annotation.v1");
        encoder.path(&self.path);
        encode_arguments(&mut encoder, &self.arguments);
        let mut pending = self
            .arguments
            .iter()
            .rev()
            .map(|arg| arg.value.as_ref())
            .collect::<Vec<_>>();
        while let Some(value) = pending.pop() {
            match value {
                StaticValueInfo::Literal(literal) => {
                    encoder.tag(0);
                    match literal {
                        AnnotationLiteralInfo::Unit => encoder.tag(0),
                        AnnotationLiteralInfo::Bool(value) => {
                            encoder.tag(1);
                            encoder.tag(u8::from(*value));
                        }
                        AnnotationLiteralInfo::Integer(value) => {
                            encoder.tag(2);
                            encoder.bytes(value.as_bytes());
                        }
                        AnnotationLiteralInfo::Float(value) => {
                            encoder.tag(3);
                            encoder.bytes(value.as_bytes());
                        }
                        AnnotationLiteralInfo::String(value) => {
                            encoder.tag(4);
                            encoder.bytes(value.as_bytes());
                        }
                        AnnotationLiteralInfo::Character(value) => {
                            encoder.tag(5);
                            encoder.number(u64::from(u32::from(*value)));
                        }
                    }
                }
                StaticValueInfo::Reference(reference) => {
                    encoder.tag(1);
                    encode_reference(&mut encoder, reference);
                }
                StaticValueInfo::Sequence { kind, elements } => {
                    encoder.tag(2);
                    encoder.tag(match kind {
                        AnnotationSequenceKind::Array => 0,
                        AnnotationSequenceKind::List => 1,
                        AnnotationSequenceKind::Set => 2,
                        AnnotationSequenceKind::Tuple => 3,
                    });
                    encoder.number(elements.len() as u64);
                    pending.extend(elements.iter().rev().map(Arc::as_ref));
                }
                StaticValueInfo::Record(fields) => {
                    encoder.tag(3);
                    encoder.number(fields.len() as u64);
                    for field in fields {
                        encoder.bytes(field.name.as_bytes());
                    }
                    pending.extend(fields.iter().rev().map(|field| field.value.as_ref()));
                }
                StaticValueInfo::Constructor {
                    kind,
                    target,
                    arguments,
                } => {
                    encoder.tag(4);
                    encoder.tag(match kind {
                        AnnotationConstructorKind::Limit => 0,
                        AnnotationConstructorKind::Trace => 1,
                    });
                    encode_reference(&mut encoder, target);
                    encode_arguments(&mut encoder, arguments);
                    pending.extend(arguments.iter().rev().map(|arg| arg.value.as_ref()));
                }
            }
        }
        encoder.finish()
    }
}

fn encode_reference(encoder: &mut CanonicalEncoder, reference: &AnnotationReferenceInfo) {
    match reference {
        AnnotationReferenceInfo::Declaration(identity) => {
            encoder.tag(0);
            encoder.fingerprint(identity.fingerprint());
        }
        AnnotationReferenceInfo::Module { package, path } => {
            encoder.tag(1);
            encoder.fingerprint(package.fingerprint());
            encoder.path(path);
        }
    }
}
fn encode_arguments(encoder: &mut CanonicalEncoder, arguments: &[AnnotationArgumentInfo]) {
    encoder.number(arguments.len() as u64);
    for argument in arguments {
        match &argument.name {
            Some(name) => {
                encoder.tag(1);
                encoder.bytes(name.as_bytes());
            }
            None => encoder.tag(0),
        }
    }
}
