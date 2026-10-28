use crate::StdType;

use super::StdRegistryValidationError;

/// Canonical compiler constructors are structural templates over their type
/// parameters. Named declaration references belong to nominal representations,
/// not a second alias-expansion graph in the registry.
pub(super) fn validate(owner: &str, root: &StdType) -> Result<(), StdRegistryValidationError> {
    let mut pending = vec![root];
    while let Some(ty) = pending.pop() {
        match ty {
            StdType::Named(_) | StdType::NamedApplied { .. } => {
                return Err(StdRegistryValidationError {
                    symbol: owner.into(),
                    reason: "canonical compiler type must be a structural template over declared parameters, not a named declaration reference".into(),
                });
            }
            StdType::Array(inner)
            | StdType::List(inner)
            | StdType::Set(inner)
            | StdType::Range(inner)
            | StdType::Slice(inner)
            | StdType::Option(inner)
            | StdType::Schema(inner)
            | StdType::Message(inner)
            | StdType::MemorySelection(inner)
            | StdType::MemoryRegion(inner)
            | StdType::ResourceHandleMemoryRegion(inner)
            | StdType::Trust { inner, .. } => pending.push(inner),
            StdType::Map { key, value } | StdType::Store { key, value } => {
                pending.extend([key.as_ref(), value.as_ref()]);
            }
            StdType::Result { ok, err } => pending.extend([ok.as_ref(), err.as_ref()]),
            StdType::Tuple(elements) => pending.extend(elements),
            StdType::Function {
                input,
                output,
                effects,
            } => {
                pending.extend(input);
                pending.push(output);
                for effect in effects {
                    pending.extend(effect.args.iter().filter_map(|arg| match arg {
                        crate::StdStaticArg::Type(ty) => Some(ty),
                        _ => None,
                    }));
                }
            }
            StdType::Record(fields) => pending.extend(fields.iter().map(|field| &field.ty)),
            StdType::Primitive(_)
            | StdType::Var(_)
            | StdType::Support(_)
            | StdType::Prompt
            | StdType::PromptPart => {}
        }
    }
    Ok(())
}
