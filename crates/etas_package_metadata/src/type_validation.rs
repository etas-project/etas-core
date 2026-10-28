use crate::{EffectRow, MetadataArtifactError, Type, TypeField, TypeKind};

pub(crate) fn type_kind_to_wire(kind: TypeKind) -> &'static str {
    match kind {
        TypeKind::Primitive => "primitive",
        TypeKind::Var => "var",
        TypeKind::Named => "named",
        TypeKind::Applied => "applied",
        TypeKind::Alias => "alias",
        TypeKind::Nominal => "nominal",
        TypeKind::Enum => "enum",
        TypeKind::Array => "array",
        TypeKind::List => "list",
        TypeKind::Map => "map",
        TypeKind::Set => "set",
        TypeKind::Range => "range",
        TypeKind::Slice => "slice",
        TypeKind::Option => "option",
        TypeKind::Result => "result",
        TypeKind::Record => "record",
        TypeKind::Tuple => "tuple",
        TypeKind::Function => "function",
        TypeKind::Handler => "handler",
        TypeKind::Trusted => "trusted",
        TypeKind::Untrusted => "untrusted",
        TypeKind::Secret => "secret",
        TypeKind::Public => "public",
        TypeKind::Sanitized => "sanitized",
        TypeKind::Prompt => "prompt",
        TypeKind::PromptPart => "prompt_part",
        TypeKind::Message => "message",
        TypeKind::MemorySelection => "memory_selection",
        TypeKind::Store => "store",
        TypeKind::MemoryRegion => "memory_region",
        TypeKind::ResourceHandle => "resource_handle",
    }
}

pub(crate) fn validate_type_shape(
    kind: TypeKind,
    name: &str,
    path: &[String],
    children: &[Type],
    fields: &[TypeField],
    effects: &Option<EffectRow>,
    produced_effects: &Option<EffectRow>,
) -> Result<(), MetadataArtifactError> {
    if matches!(kind, TypeKind::Record | TypeKind::Enum) {
        let mut names = std::collections::BTreeSet::new();
        for field in fields {
            if field.name.is_empty() || !names.insert(&field.name) {
                return Err(invalid(
                    "record fields and enum variants require unique nonempty names",
                ));
            }
        }
    }
    match kind {
        TypeKind::Primitive | TypeKind::Var | TypeKind::ResourceHandle if name.is_empty() => {
            Err(invalid(format!(
                "type kind `{}` requires name",
                type_kind_to_wire(kind)
            )))
        }
        TypeKind::Named
        | TypeKind::Applied
        | TypeKind::Alias
        | TypeKind::Nominal
        | TypeKind::Enum
            if path.is_empty() =>
        {
            Err(invalid(format!(
                "type kind `{}` requires path",
                type_kind_to_wire(kind)
            )))
        }
        TypeKind::Applied if children.is_empty() => {
            Err(invalid("applied type requires at least one argument child"))
        }
        TypeKind::Alias if children.len() != 1 => {
            Err(invalid("alias type requires exactly one target child"))
        }
        TypeKind::Nominal if children.len() > 1 => Err(invalid(
            "nominal type can carry at most one representation child",
        )),
        TypeKind::Enum if !children.is_empty() => Err(invalid("enum type cannot use children")),
        TypeKind::Enum
            if fields
                .iter()
                .any(|variant| !matches!(variant.ty.kind, TypeKind::Tuple | TypeKind::Record)) =>
        {
            Err(invalid(
                "enum variant payload must be positional or named fields",
            ))
        }
        TypeKind::Array
        | TypeKind::List
        | TypeKind::Set
        | TypeKind::Range
        | TypeKind::Slice
        | TypeKind::Option
        | TypeKind::Trusted
        | TypeKind::Untrusted
        | TypeKind::Secret
        | TypeKind::Public
        | TypeKind::Sanitized
        | TypeKind::Message
        | TypeKind::MemorySelection
        | TypeKind::MemoryRegion
            if children.len() != 1 =>
        {
            Err(invalid(format!(
                "type kind `{}` requires exactly one child",
                type_kind_to_wire(kind)
            )))
        }
        TypeKind::Map | TypeKind::Result | TypeKind::Store if children.len() != 2 => {
            Err(invalid(format!(
                "type kind `{}` requires exactly two children",
                type_kind_to_wire(kind)
            )))
        }
        TypeKind::Function if children.is_empty() => {
            Err(invalid("function type requires an output child"))
        }
        TypeKind::Handler if effects.is_none() => {
            Err(invalid("handler type requires handled effects"))
        }
        TypeKind::Handler if children.len() > 1 => {
            Err(invalid("handler type can carry at most one result child"))
        }
        TypeKind::Record if !children.is_empty() => Err(invalid("record type cannot use children")),
        kind if !matches!(kind, TypeKind::Record | TypeKind::Enum) && !fields.is_empty() => {
            Err(invalid(format!(
                "type kind `{}` cannot use record fields",
                type_kind_to_wire(kind)
            )))
        }
        kind if !matches!(kind, TypeKind::Function | TypeKind::Handler) && effects.is_some() => {
            Err(invalid(format!(
                "type kind `{}` cannot use effects",
                type_kind_to_wire(kind)
            )))
        }
        kind if kind != TypeKind::Handler && produced_effects.is_some() => Err(invalid(format!(
            "type kind `{}` cannot use handler produced effects",
            type_kind_to_wire(kind)
        ))),
        _ => Ok(()),
    }
}

fn invalid(message: impl Into<String>) -> MetadataArtifactError {
    MetadataArtifactError::invalid(crate::PACKAGE_METADATA_FILE, message)
}
