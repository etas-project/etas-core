use etas_code::descriptor::*;

use super::{
    identities::{decode_key, encode_key},
    invalid, required, schema as p,
};
use crate::MetadataArtifactError as Error;

macro_rules! enum_codec {
    ($ty:ident, $encode:ident, $decode:ident, $($variant:ident = $id:literal),+ $(,)?) => {
        fn $encode(value: $ty) -> u32 { match value { $($ty::$variant => $id),+ } }
        fn $decode(value: u32) -> Result<$ty, Error> { match value { $($id => Ok($ty::$variant)),+, _ => Err(invalid(concat!("unknown descriptor ", stringify!($ty)))) } }
    };
}

enum_codec!(
    PrimitiveInfo,
    encode_primitive,
    decode_primitive,
    Unit = 1,
    Never = 2,
    Bool = 3,
    I8 = 4,
    I16 = 5,
    I32 = 6,
    I64 = 7,
    I128 = 8,
    Isize = 9,
    U8 = 10,
    U16 = 11,
    U32 = 12,
    U64 = 13,
    U128 = 14,
    Usize = 15,
    F32 = 16,
    F64 = 17,
    Char = 18,
    String = 19,
    Bytes = 20
);
enum_codec!(
    BuiltinTypeInfo,
    encode_builtin,
    decode_builtin,
    Array = 1,
    List = 2,
    Map = 3,
    Set = 4,
    Range = 5,
    Slice = 6,
    Option = 7,
    Result = 8,
    Schema = 9,
    Prompt = 10,
    PromptPart = 11,
    Message = 12,
    MemorySelection = 13,
    Store = 14,
    MemoryRegion = 15,
    MemoryRegionHandle = 16,
    ExternalToolHandle = 17,
    Trusted = 18,
    Untrusted = 19,
    Secret = 20,
    Public = 21,
    Sanitized = 22
);

fn refs(values: &[TypeInfoRef]) -> Vec<u32> {
    values.iter().map(|value| value.0).collect()
}
fn encode_fields(fields: &[FieldInfo]) -> p::Fields {
    p::Fields {
        values: fields
            .iter()
            .map(|field| p::Field {
                name: field.name.clone(),
                ty: field.ty.0,
            })
            .collect(),
    }
}
fn decode_fields(fields: p::Fields) -> Vec<FieldInfo> {
    fields
        .values
        .into_iter()
        .map(|field| FieldInfo {
            name: field.name,
            ty: TypeInfoRef(field.ty),
        })
        .collect()
}

pub(super) fn encode(ty: &TypeInfo) -> p::Type {
    use p::ty::Kind as K;
    let kind = match ty {
        TypeInfo::Primitive(value) => K::Primitive(encode_primitive(*value)),
        TypeInfo::Builtin {
            constructor,
            arguments,
        } => K::Builtin(p::Builtin {
            constructor: encode_builtin(*constructor),
            arguments: refs(arguments),
        }),
        TypeInfo::Parameter(value) => K::Parameter(value.0),
        TypeInfo::Tuple(values) => K::Tuple(p::Refs {
            values: refs(values),
        }),
        TypeInfo::Record(fields) => K::Record(encode_fields(fields)),
        TypeInfo::Nominal {
            definition,
            parameters,
            representation,
        } => K::Nominal(declaration(
            definition,
            parameters,
            representation.map(|r| r.0),
            Vec::new(),
        )),
        TypeInfo::Alias {
            definition,
            parameters,
            target,
        } => K::Alias(declaration(
            definition,
            parameters,
            Some(target.0),
            Vec::new(),
        )),
        TypeInfo::Enum {
            definition,
            parameters,
            variants,
        } => K::Enum(declaration(
            definition,
            parameters,
            None,
            variants
                .iter()
                .map(|variant| p::Variant {
                    name: variant.name.clone(),
                    payload: Some(match &variant.payload {
                        VariantPayloadInfo::None => p::variant::Payload::None(true),
                        VariantPayloadInfo::Positional(values) => {
                            p::variant::Payload::Positional(p::Refs {
                                values: refs(values),
                            })
                        }
                        VariantPayloadInfo::Named(fields) => {
                            p::variant::Payload::Named(encode_fields(fields))
                        }
                    }),
                })
                .collect(),
        )),
        TypeInfo::Applied {
            constructor,
            arguments,
        } => K::Applied(p::Applied {
            constructor: constructor.0,
            arguments: refs(arguments),
        }),
        TypeInfo::Function(signature) => K::Function(p::Signature {
            generics: signature.generics.iter().map(|r| r.0).collect(),
            parameters: signature
                .parameters
                .iter()
                .map(|parameter| p::Parameter {
                    name: parameter.name.clone(),
                    ty: parameter.ty.0,
                })
                .collect(),
            output: signature.output.0,
            declared_output: signature.declared_output.map(|r| r.0),
            effects: signature.effects.map(|r| r.0),
        }),
        TypeInfo::Handler {
            handled,
            produced,
            result,
        } => K::Handler(p::Handler {
            handled: handled.0,
            produced: produced.map(|r| r.0),
            result: result.map(|r| r.0),
        }),
    };
    p::Type { kind: Some(kind) }
}

fn declaration(
    definition: &etas_code::DefinitionKey,
    parameters: &[GenericParameterRef],
    representation: Option<u32>,
    variants: Vec<p::Variant>,
) -> p::Declaration {
    p::Declaration {
        definition: Some(encode_key(definition)),
        parameters: parameters.iter().map(|r| r.0).collect(),
        representation,
        variants,
    }
}

pub(super) fn decode(ty: p::Type) -> Result<TypeInfo, Error> {
    use p::ty::Kind as K;
    Ok(match required(ty.kind, "type kind")? {
        K::Primitive(value) => TypeInfo::Primitive(decode_primitive(value)?),
        K::Builtin(value) => TypeInfo::Builtin {
            constructor: decode_builtin(value.constructor)?,
            arguments: value.arguments.into_iter().map(TypeInfoRef).collect(),
        },
        K::Parameter(value) => TypeInfo::Parameter(GenericParameterRef(value)),
        K::Tuple(value) => TypeInfo::Tuple(value.values.into_iter().map(TypeInfoRef).collect()),
        K::Record(value) => TypeInfo::Record(decode_fields(value)),
        K::Nominal(value) => {
            if !value.variants.is_empty() {
                return Err(invalid("nominal descriptor contains enum variants"));
            }
            TypeInfo::Nominal {
                definition: decode_key(required(value.definition, "nominal identity")?)?,
                parameters: value
                    .parameters
                    .into_iter()
                    .map(GenericParameterRef)
                    .collect(),
                representation: value.representation.map(TypeInfoRef),
            }
        }
        K::Alias(value) => {
            if !value.variants.is_empty() {
                return Err(invalid("alias descriptor contains enum variants"));
            }
            TypeInfo::Alias {
                definition: decode_key(required(value.definition, "alias identity")?)?,
                parameters: value
                    .parameters
                    .into_iter()
                    .map(GenericParameterRef)
                    .collect(),
                target: TypeInfoRef(required(value.representation, "alias target")?),
            }
        }
        K::Enum(value) => {
            if value.representation.is_some() {
                return Err(invalid("enum descriptor contains nominal representation"));
            }
            TypeInfo::Enum {
                definition: decode_key(required(value.definition, "enum identity")?)?,
                parameters: value
                    .parameters
                    .into_iter()
                    .map(GenericParameterRef)
                    .collect(),
                variants: value
                    .variants
                    .into_iter()
                    .map(|variant| {
                        Ok(VariantInfo {
                            name: variant.name,
                            payload: match required(variant.payload, "enum payload form")? {
                                p::variant::Payload::None(true) => VariantPayloadInfo::None,
                                p::variant::Payload::None(false) => {
                                    return Err(invalid("invalid nullary payload marker"));
                                }
                                p::variant::Payload::Positional(values) => {
                                    VariantPayloadInfo::Positional(
                                        values.values.into_iter().map(TypeInfoRef).collect(),
                                    )
                                }
                                p::variant::Payload::Named(fields) => {
                                    VariantPayloadInfo::Named(decode_fields(fields))
                                }
                            },
                        })
                    })
                    .collect::<Result<_, Error>>()?,
            }
        }
        K::Applied(value) => TypeInfo::Applied {
            constructor: TypeInfoRef(value.constructor),
            arguments: value.arguments.into_iter().map(TypeInfoRef).collect(),
        },
        K::Function(value) => TypeInfo::Function(SignatureInfo {
            generics: value
                .generics
                .into_iter()
                .map(GenericParameterRef)
                .collect(),
            parameters: value
                .parameters
                .into_iter()
                .map(|parameter| ParameterInfo {
                    name: parameter.name,
                    ty: TypeInfoRef(parameter.ty),
                })
                .collect(),
            output: TypeInfoRef(value.output),
            declared_output: value.declared_output.map(TypeInfoRef),
            effects: value.effects.map(EffectRowInfoRef),
        }),
        K::Handler(value) => TypeInfo::Handler {
            handled: EffectRowInfoRef(value.handled),
            produced: value.produced.map(EffectRowInfoRef),
            result: value.result.map(TypeInfoRef),
        },
    })
}
