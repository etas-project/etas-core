use std::{collections::BTreeSet, fmt};

use crate::{EffectActionArgKind, StdDecl, StdEffectRef, StdRegistry, StdStaticArg, StdType};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StdRegistryValidationError {
    pub symbol: String,
    pub reason: String,
}

impl fmt::Display for StdRegistryValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid standard declaration `{}`: {}",
            self.symbol, self.reason
        )
    }
}

impl std::error::Error for StdRegistryValidationError {}

pub(crate) fn validate_registry(registry: &StdRegistry) -> Result<(), StdRegistryValidationError> {
    for symbol in registry.symbols() {
        match &symbol.decl {
            StdDecl::Flow(flow) => {
                let generics = flow
                    .type_params
                    .iter()
                    .map(|param| param.name.as_str())
                    .collect::<BTreeSet<_>>();
                validate_effect_row(
                    registry,
                    &symbol.qualified_path,
                    &flow.public_effects,
                    EffectReferenceKind::Effect,
                    &generics,
                )?;
                validate_effect_row(
                    registry,
                    &symbol.qualified_path,
                    &flow.requested_actions,
                    EffectReferenceKind::Action,
                    &generics,
                )?;
            }
            StdDecl::Tool(tool) => validate_effect_row(
                registry,
                &symbol.qualified_path,
                &tool.effects,
                EffectReferenceKind::Effect,
                &BTreeSet::new(),
            )?,
            _ => {}
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum EffectReferenceKind {
    Effect,
    Action,
}

fn validate_effect_row(
    registry: &StdRegistry,
    owner: &[String],
    effects: &[StdEffectRef],
    kind: EffectReferenceKind,
    generics: &BTreeSet<&str>,
) -> Result<(), StdRegistryValidationError> {
    for effect in effects {
        validate_effect_ref(registry, owner, effect, kind, generics)?;
    }
    Ok(())
}

fn validate_effect_ref(
    registry: &StdRegistry,
    owner: &[String],
    effect: &StdEffectRef,
    kind: EffectReferenceKind,
    generics: &BTreeSet<&str>,
) -> Result<(), StdRegistryValidationError> {
    let owner_name = owner.join(".");
    if effect.path.is_empty() || effect.path.iter().any(String::is_empty) {
        return invalid(owner_name, "effect reference has an empty path");
    }

    match kind {
        EffectReferenceKind::Effect => {
            let [name] = effect.path.as_slice() else {
                return invalid(
                    owner_name,
                    format!(
                        "public effect `{}` must use its logical effect identity",
                        effect.path.join(".")
                    ),
                );
            };
            let declaration = registry.symbols().find_map(|symbol| {
                let StdDecl::Effect(declaration) = &symbol.decl else {
                    return None;
                };
                (declaration.name == *name).then_some(declaration)
            });
            let Some(declaration) = declaration else {
                return invalid(owner_name, format!("unknown effect `{name}`"));
            };
            if effect.args.len() != declaration.params.len() {
                return invalid(
                    owner_name,
                    format!(
                        "effect `{name}` expects {} static arguments but received {}",
                        declaration.params.len(),
                        effect.args.len()
                    ),
                );
            }
            for arg in &effect.args {
                if !matches!(arg, StdStaticArg::Type(_) | StdStaticArg::Wildcard) {
                    return invalid(
                        owner_name.clone(),
                        format!("effect `{name}` requires type static arguments"),
                    );
                }
                validate_generic_bindings(&owner_name, arg, generics)?;
            }
        }
        EffectReferenceKind::Action => {
            let [action_owner, action_name] = effect.path.as_slice() else {
                return invalid(
                    owner_name,
                    format!(
                        "requested action `{}` must use owner and action segments",
                        effect.path.join(".")
                    ),
                );
            };
            let declaration = registry.symbols().find_map(|symbol| {
                let StdDecl::EffectAction(declaration) = &symbol.decl else {
                    return None;
                };
                (declaration.owner == *action_owner && declaration.name == *action_name)
                    .then_some(declaration)
            });
            let Some(declaration) = declaration else {
                return invalid(
                    owner_name,
                    format!("unknown action `{action_owner}.{action_name}`"),
                );
            };
            if effect.args.len() != declaration.effect_args.len() {
                return invalid(
                    owner_name,
                    format!(
                        "action `{action_owner}.{action_name}` expects {} static selectors but received {}",
                        declaration.effect_args.len(),
                        effect.args.len()
                    ),
                );
            }
            for (arg, expected) in effect.args.iter().zip(&declaration.effect_args) {
                if !static_arg_matches_kind(arg, expected) {
                    return invalid(
                        owner_name.clone(),
                        format!(
                            "action `{action_owner}.{action_name}` has a static selector with the wrong kind"
                        ),
                    );
                }
                if let StdStaticArg::Path(path) = arg
                    && registry.lookup_qualified(path).is_none()
                {
                    return invalid(
                        owner_name.clone(),
                        format!(
                            "action `{action_owner}.{action_name}` references unknown static resource `{}`",
                            path.join(".")
                        ),
                    );
                }
                validate_generic_bindings(&owner_name, arg, generics)?;
            }
        }
    }
    Ok(())
}

fn static_arg_matches_kind(arg: &StdStaticArg, expected: &EffectActionArgKind) -> bool {
    if matches!(arg, StdStaticArg::Wildcard) {
        return true;
    }
    match expected {
        EffectActionArgKind::Type => matches!(arg, StdStaticArg::Type(_)),
        EffectActionArgKind::MemoryPlace | EffectActionArgKind::StaticResourcePath { .. } => {
            matches!(arg, StdStaticArg::Path(_))
        }
        EffectActionArgKind::StringPattern => matches!(
            arg,
            StdStaticArg::Path(_) | StdStaticArg::String(_) | StdStaticArg::Int(_)
        ),
    }
}

fn validate_generic_bindings(
    owner: &str,
    arg: &StdStaticArg,
    generics: &BTreeSet<&str>,
) -> Result<(), StdRegistryValidationError> {
    let StdStaticArg::Type(ty) = arg else {
        return Ok(());
    };
    let mut referenced = BTreeSet::new();
    collect_type_vars(ty, &mut referenced);
    for name in referenced {
        if !generics.contains(name.as_str()) {
            return invalid(
                owner.to_owned(),
                format!("static selector references undeclared generic `{name}`"),
            );
        }
    }
    Ok(())
}

fn collect_type_vars(ty: &StdType, output: &mut BTreeSet<String>) {
    match ty {
        StdType::Var(name) => {
            output.insert(name.clone());
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
        | StdType::ResourceHandleMemoryRegion(inner) => collect_type_vars(inner, output),
        StdType::Map { key, value } | StdType::Store { key, value } => {
            collect_type_vars(key, output);
            collect_type_vars(value, output);
        }
        StdType::Result { ok, err } => {
            collect_type_vars(ok, output);
            collect_type_vars(err, output);
        }
        StdType::Tuple(elements) | StdType::NamedApplied { args: elements, .. } => {
            for element in elements {
                collect_type_vars(element, output);
            }
        }
        StdType::Trust { inner, .. } => collect_type_vars(inner, output),
        StdType::Record(fields) => {
            for field in fields {
                collect_type_vars(&field.ty, output);
            }
        }
        StdType::Primitive(_)
        | StdType::Support(_)
        | StdType::Prompt
        | StdType::PromptPart
        | StdType::Named(_) => {}
    }
}

fn invalid<T>(
    symbol: impl Into<String>,
    reason: impl Into<String>,
) -> Result<T, StdRegistryValidationError> {
    Err(StdRegistryValidationError {
        symbol: symbol.into(),
        reason: reason.into(),
    })
}
