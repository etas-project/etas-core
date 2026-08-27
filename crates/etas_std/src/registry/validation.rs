use std::{collections::BTreeSet, fmt};

use crate::{
    EffectActionArgKind, StdDecl, StdEffectRef, StdGenericParam, StdImplFact, StdRegistry,
    StdSpecRef, StdStaticArg, StdType, TypeDeclKind,
};

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
    validate_unique_symbols(registry)?;
    for symbol in registry.symbols() {
        match &symbol.decl {
            StdDecl::Type(declaration) => {
                validate_generic_params(registry, &symbol.qualified_path, &declaration.params)?;
                let generics = generic_names(&declaration.params);
                if let Some(representation) = &declaration.representation {
                    validate_type_expr(
                        registry,
                        &symbol.qualified_path.join("."),
                        representation,
                        &generics,
                    )?;
                }
            }
            StdDecl::Effect(effect) => {
                validate_unique_names(&symbol.qualified_path, "effect parameter", &effect.params)?;
            }
            StdDecl::Flow(flow) => {
                validate_generic_params(registry, &symbol.qualified_path, &flow.type_params)?;
                let generics = generic_names(&flow.type_params);
                let owner = symbol.qualified_path.join(".");
                for ty in flow.params.iter().chain(std::iter::once(&flow.output)) {
                    validate_type_expr(registry, &owner, ty, &generics)?;
                }
                if let Some(method) = &flow.source_method {
                    validate_type_expr(registry, &owner, &method.receiver, &generics)?;
                }
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
            StdDecl::Tool(tool) => {
                let generics = BTreeSet::new();
                let owner = symbol.qualified_path.join(".");
                for ty in tool.params.iter().chain(std::iter::once(&tool.output)) {
                    validate_type_expr(registry, &owner, ty, &generics)?;
                }
                validate_effect_row(
                    registry,
                    &symbol.qualified_path,
                    &tool.effects,
                    EffectReferenceKind::Effect,
                    &generics,
                )?;
            }
            StdDecl::EffectAction(action) => {
                validate_action_declaration(registry, &symbol.qualified_path, action)?;
            }
            StdDecl::Value(value) => validate_type_expr(
                registry,
                &symbol.qualified_path.join("."),
                &value.ty,
                &BTreeSet::new(),
            )?,
            StdDecl::Requirement(_) => {}
        }
    }
    validate_spec_impls(registry)?;
    Ok(())
}

fn generic_names(params: &[StdGenericParam]) -> BTreeSet<&str> {
    params.iter().map(|param| param.name.as_str()).collect()
}

fn validate_unique_symbols(registry: &StdRegistry) -> Result<(), StdRegistryValidationError> {
    let mut paths = BTreeSet::new();
    for symbol in registry.symbols() {
        if !paths.insert(symbol.qualified_path.clone()) {
            return invalid(
                symbol.qualified_path.join("."),
                "duplicate qualified standard symbol",
            );
        }
    }
    Ok(())
}

fn validate_unique_names(
    owner: &[String],
    kind: &str,
    names: &[String],
) -> Result<(), StdRegistryValidationError> {
    let mut unique = BTreeSet::new();
    for name in names {
        if name.is_empty() {
            return invalid(owner.join("."), format!("{kind} name must not be empty"));
        }
        if !unique.insert(name.as_str()) {
            return invalid(owner.join("."), format!("duplicate {kind} `{name}`"));
        }
    }
    Ok(())
}

fn validate_generic_params(
    registry: &StdRegistry,
    owner: &[String],
    params: &[StdGenericParam],
) -> Result<(), StdRegistryValidationError> {
    let names = params
        .iter()
        .map(|param| param.name.clone())
        .collect::<Vec<_>>();
    validate_unique_names(owner, "generic parameter", &names)?;
    let generics = generic_names(params);
    for param in params {
        for bound in &param.bounds {
            validate_spec_ref(registry, &owner.join("."), bound, &generics)?;
        }
    }
    Ok(())
}

fn validate_action_declaration(
    registry: &StdRegistry,
    path: &[String],
    action: &crate::EffectActionDecl,
) -> Result<(), StdRegistryValidationError> {
    let owner = path.join(".");
    if action.effect_args.len() != action.selector_param_names.len() {
        return invalid(
            owner,
            format!(
                "action selector names length {} does not match selector kinds length {}",
                action.selector_param_names.len(),
                action.effect_args.len()
            ),
        );
    }
    validate_generic_params(registry, path, &action.type_params)?;
    let generics = generic_names(&action.type_params);
    for (kind, selector_name) in action.effect_args.iter().zip(&action.selector_param_names) {
        if matches!(kind, EffectActionArgKind::Type)
            && (selector_name.is_empty() || !generics.contains(selector_name.as_str()))
        {
            return invalid(
                path.join("."),
                format!(
                    "type selector `{selector_name}` must name a declared action generic parameter"
                ),
            );
        }
    }
    for ty in action.params.iter().chain(std::iter::once(&action.output)) {
        validate_type_expr(registry, &owner, ty, &generics)?;
    }
    Ok(())
}

fn validate_spec_impls(registry: &StdRegistry) -> Result<(), StdRegistryValidationError> {
    let mut implementations = Vec::new();
    for implementation in registry.spec_impls() {
        let owner = format!(
            "impl {:?} ~ {}",
            implementation.self_type,
            implementation.spec.path.join(".")
        );
        if implementations.contains(&implementation) {
            return invalid(owner, "duplicate standard spec implementation");
        }
        implementations.push(implementation);
        validate_impl_type(registry, &owner, implementation)?;
        validate_spec_ref(registry, &owner, &implementation.spec, &BTreeSet::new())?;
    }
    Ok(())
}

fn validate_impl_type(
    registry: &StdRegistry,
    owner: &str,
    implementation: &StdImplFact,
) -> Result<(), StdRegistryValidationError> {
    validate_type_expr(registry, owner, &implementation.self_type, &BTreeSet::new())
}

fn validate_spec_ref(
    registry: &StdRegistry,
    owner: &str,
    spec: &StdSpecRef,
    generics: &BTreeSet<&str>,
) -> Result<(), StdRegistryValidationError> {
    let Some(symbol) = registry.lookup_qualified(&spec.path) else {
        return invalid(
            owner.to_owned(),
            format!("unknown spec `{}`", spec.path.join(".")),
        );
    };
    let StdDecl::Type(declaration) = &symbol.decl else {
        return invalid(
            owner.to_owned(),
            format!("bound `{}` does not name a type spec", spec.path.join(".")),
        );
    };
    if declaration.kind != TypeDeclKind::Spec {
        return invalid(
            owner.to_owned(),
            format!("bound `{}` does not name a type spec", spec.path.join(".")),
        );
    }
    if spec.args.len() != declaration.params.len() {
        return invalid(
            owner.to_owned(),
            format!(
                "spec `{}` expects {} argument(s) but received {}",
                spec.path.join("."),
                declaration.params.len(),
                spec.args.len()
            ),
        );
    }
    for arg in &spec.args {
        validate_type_expr(registry, owner, arg, generics)?;
    }
    Ok(())
}

fn validate_type_expr(
    registry: &StdRegistry,
    owner: &str,
    ty: &StdType,
    generics: &BTreeSet<&str>,
) -> Result<(), StdRegistryValidationError> {
    match ty {
        StdType::Named(name) => validate_named_type(registry, owner, name, 0),
        StdType::NamedApplied { name, args } => {
            validate_named_type(registry, owner, name, args.len())?;
            for arg in args {
                validate_type_expr(registry, owner, arg, generics)?;
            }
            Ok(())
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
        | StdType::Trust { inner, .. } => validate_type_expr(registry, owner, inner, generics),
        StdType::Map { key, value } | StdType::Store { key, value } => {
            validate_type_expr(registry, owner, key, generics)?;
            validate_type_expr(registry, owner, value, generics)
        }
        StdType::Result { ok, err } => {
            validate_type_expr(registry, owner, ok, generics)?;
            validate_type_expr(registry, owner, err, generics)
        }
        StdType::Tuple(elements) => {
            for element in elements {
                validate_type_expr(registry, owner, element, generics)?;
            }
            Ok(())
        }
        StdType::Record(fields) => {
            for field in fields {
                validate_type_expr(registry, owner, &field.ty, generics)?;
            }
            Ok(())
        }
        StdType::Var(name) if generics.contains(name.as_str()) => Ok(()),
        StdType::Var(name) => invalid(
            owner.to_owned(),
            format!("type expression references undeclared generic `{name}`"),
        ),
        StdType::Primitive(_) | StdType::Support(_) | StdType::Prompt | StdType::PromptPart => {
            Ok(())
        }
    }
}

fn validate_named_type(
    registry: &StdRegistry,
    owner: &str,
    name: &str,
    arity: usize,
) -> Result<(), StdRegistryValidationError> {
    let declaration = if name.contains('.') {
        registry
            .lookup_qualified(&name.split('.').collect::<Vec<_>>())
            .and_then(|symbol| match &symbol.decl {
                StdDecl::Type(declaration) => Some(declaration),
                _ => None,
            })
    } else {
        let mut matches = registry.symbols().filter_map(|symbol| match &symbol.decl {
            StdDecl::Type(declaration) if declaration.name == name => Some(declaration),
            _ => None,
        });
        let declaration = matches.next();
        if declaration.is_some() && matches.next().is_some() {
            return invalid(
                owner.to_owned(),
                format!("standard type name `{name}` is ambiguous"),
            );
        }
        declaration
    };
    let Some(declaration) = declaration else {
        return invalid(owner.to_owned(), format!("unknown standard type `{name}`"));
    };
    if declaration.kind == TypeDeclKind::Spec {
        return invalid(
            owner.to_owned(),
            format!("spec `{name}` cannot be used as a value type"),
        );
    }
    if declaration.params.len() != arity {
        return invalid(
            owner.to_owned(),
            format!(
                "standard type `{name}` expects {} argument(s) but received {arity}",
                declaration.params.len()
            ),
        );
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
                if let StdStaticArg::Type(ty) = arg {
                    validate_type_expr(registry, &owner_name, ty, generics)?;
                }
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
                if let StdStaticArg::Type(ty) = arg {
                    validate_type_expr(registry, &owner_name, ty, generics)?;
                }
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

fn invalid<T>(
    symbol: impl Into<String>,
    reason: impl Into<String>,
) -> Result<T, StdRegistryValidationError> {
    Err(StdRegistryValidationError {
        symbol: symbol.into(),
        reason: reason.into(),
    })
}
