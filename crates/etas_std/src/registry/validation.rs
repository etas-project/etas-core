use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use crate::{
    EffectActionArgKind, RequirementKind, RequirementSemantics, StdDecl, StdEffectRef,
    StdGenericParam, StdImplFact, StdRegistry, StdSpecRef, StdStaticArg, StdSymbolKind, StdType,
    TypeDeclKind,
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
    validate_stable_ids(registry)?;
    let logical_effects = LogicalEffectIndex::build(registry)?;
    let mut code_roles = BTreeSet::new();
    let mut action_roles = BTreeSet::new();
    for symbol in registry.symbols() {
        super::enum_constructor::validate(registry, symbol)?;
        if let Some(argument) = registry.memory_place_result_argument(symbol.id)
            && !matches!(&symbol.decl, StdDecl::Flow(flow) if matches!(flow.params.get(argument), Some(StdType::Store { .. })))
        {
            return Err(StdRegistryValidationError {
                symbol: symbol.qualified_path.join("."),
                reason:
                    "intrinsic memory provenance requires the same Store argument in every alias"
                        .into(),
            });
        }
        match &symbol.decl {
            StdDecl::Type(declaration) => {
                if let Some(role) = declaration.code_role
                    && (!code_roles.insert(role)
                        || symbol.kind != StdSymbolKind::Type
                        || declaration.kind != TypeDeclKind::Struct
                        || declaration.construction != crate::StdTypeConstruction::IntrinsicOnly
                        || declaration.representation.is_some()
                        || declaration.canonical_type.is_some()
                        || declaration.params.len() != role.type_parameter_count())
                {
                    return Err(StdRegistryValidationError { symbol: symbol.qualified_path.join("."),
                        reason: "code descriptor role requires a unique opaque nominal type with matching arity".into() });
                }
                validate_generic_params(registry, &symbol.qualified_path, &declaration.params)?;
                if let Some(item) = declaration.iterable_item
                    && (declaration.kind != TypeDeclKind::Support
                        || item
                            .parameters()
                            .any(|parameter| parameter >= declaration.params.len()))
                {
                    return Err(StdRegistryValidationError {
                        symbol: symbol.qualified_path.join("."),
                        reason: "iterable item must reference declared support type parameters"
                            .into(),
                    });
                }
                let generics = generic_names(&declaration.params);
                if let Some(ty) = &declaration.canonical_type {
                    if declaration.kind != TypeDeclKind::Support
                        || declaration.representation.is_some()
                    {
                        return Err(StdRegistryValidationError {
                            symbol: symbol.qualified_path.join("."),
                            reason:
                                "canonical compiler type requires a non-nominal support declaration"
                                    .into(),
                        });
                    }
                    super::canonical_type::validate(&symbol.qualified_path.join("."), ty)?;
                    validate_type_expr(registry, &symbol.qualified_path.join("."), ty, &generics)?;
                }
                let mut projected_names = BTreeSet::new();
                for field in &declaration.projected_fields {
                    if field.name.is_empty() || !projected_names.insert(&field.name) {
                        return Err(StdRegistryValidationError {
                            symbol: symbol.qualified_path.join("."),
                            reason: "projected fields require unique nonempty names".into(),
                        });
                    }
                    validate_type_expr(
                        registry,
                        &symbol.qualified_path.join("."),
                        &field.ty,
                        &generics,
                    )?;
                }
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
                validate_effect_extensions(&logical_effects, &symbol.qualified_path, effect)?;
            }
            StdDecl::Flow(flow) => {
                if let Some(descriptor) = &symbol.intrinsic {
                    super::action_forwarder::validate(registry, flow, descriptor)?;
                    super::default_action::validate(registry, flow, descriptor)?;
                    super::collection::validate(flow, descriptor)?;
                }
                validate_generic_params(registry, &symbol.qualified_path, &flow.type_params)?;
                let generics = generic_names(&flow.type_params);
                let owner = symbol.qualified_path.join(".");
                for ty in flow.params.iter().chain(std::iter::once(&flow.output)) {
                    validate_type_expr(registry, &owner, ty, &generics)?;
                }
                if let Some(method) = &flow.source_method {
                    if method.operation == crate::FlowSourceMethodOperation::CheckedIntrinsic
                        && (symbol.intrinsic.is_none()
                            || method.kind != crate::FlowSourceMethodKind::ValueMethod
                            || flow.params.first() != Some(&method.receiver)
                            || symbol.intrinsic.as_ref().is_some_and(|descriptor| {
                                descriptor.purity != crate::IntrinsicPurity::Pure
                                    || descriptor.dispatch == crate::IntrinsicDispatch::LoweringOnly
                            })
                            || !flow.public_effects.is_empty()
                            || !flow.requested_actions.is_empty())
                    {
                        return Err(StdRegistryValidationError {
                            symbol: symbol.qualified_path.join("."),
                            reason: "checked intrinsic method requires a pure executable descriptor, an effect-free contract and a matching first receiver parameter".into(),
                        });
                    }
                    validate_type_expr(registry, &owner, &method.receiver, &generics)?;
                    super::checked_index::validate(&owner, flow, method)?;
                }
                validate_effect_row(
                    registry,
                    &logical_effects,
                    &symbol.qualified_path,
                    &flow.public_effects,
                    EffectReferenceKind::Effect,
                    &generics,
                )?;
                validate_effect_row(
                    registry,
                    &logical_effects,
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
                    &logical_effects,
                    &symbol.qualified_path,
                    &tool.effects,
                    EffectReferenceKind::Effect,
                    &generics,
                )?;
                validate_tool_requirements(registry, &symbol.qualified_path, &tool.requirements)?;
            }
            StdDecl::EffectAction(action) => {
                if let Some(role) = action.compiler_role {
                    let valid_shape = action.type_params.len() == 1
                        && action.params.len() == 2
                        && action.params[0] == StdType::Prompt
                        && action.params[1] == StdType::Schema(Box::new(action.output.clone()))
                        && matches!(&action.output, StdType::Var(name) if *name == action.type_params[0].name)
                        && matches!(
                            action.effect_args.as_slice(),
                            [
                                EffectActionArgKind::StaticResourcePath { ty: "Agent" },
                                EffectActionArgKind::Type
                            ]
                        )
                        && action.runtime_requirement
                            == Some(crate::StdRuntimeRequirement::Agentic)
                        && action.stable_id.is_some();
                    if !action_roles.insert(role) || !valid_shape {
                        return Err(StdRegistryValidationError {
                            symbol: symbol.qualified_path.join("."),
                            reason: "agent inference role requires a unique action with Prompt/Schema<O> ABI and callable/output selectors".into(),
                        });
                    }
                }
                validate_action_declaration(registry, &symbol.qualified_path, action)?;
                if let Some(effects) = &action.default_handler_effects {
                    validate_effect_row(
                        registry,
                        &logical_effects,
                        &symbol.qualified_path,
                        effects,
                        EffectReferenceKind::Effect,
                        &generic_names(&action.type_params),
                    )?;
                }
                validate_action_owner(&logical_effects, &symbol.qualified_path, action)?;
            }
            StdDecl::Value(value) => validate_type_expr(
                registry,
                &symbol.qualified_path.join("."),
                &value.ty,
                &BTreeSet::new(),
            )?,
            StdDecl::Requirement(requirement) => {
                validate_requirement(registry, &symbol.qualified_path, requirement)?;
            }
        }
    }
    validate_spec_impls(registry)?;
    validate_effect_extension_cycles(&logical_effects)?;
    Ok(())
}

struct LogicalEffectIndex<'a> {
    effects: BTreeMap<String, &'a crate::EffectDecl>,
    actions: BTreeMap<(String, String), &'a crate::EffectActionDecl>,
}

impl<'a> LogicalEffectIndex<'a> {
    fn build(registry: &'a StdRegistry) -> Result<Self, StdRegistryValidationError> {
        let mut effects = BTreeMap::new();
        let mut actions = BTreeMap::new();
        for symbol in registry.symbols() {
            match &symbol.decl {
                StdDecl::Effect(effect) => {
                    validate_decl_identity(symbol, StdSymbolKind::Effect, &effect.name, "effect")?;
                    if let Some(existing) = effects.insert(effect.name.clone(), effect) {
                        return invalid(
                            symbol.qualified_path.join("."),
                            format!(
                                "duplicate logical effect identity `{}`; first declaration has stable id {:?}",
                                effect.name, existing.stable_id
                            ),
                        );
                    }
                }
                StdDecl::EffectAction(action) => {
                    validate_decl_identity(
                        symbol,
                        StdSymbolKind::EffectAction,
                        &action.name,
                        "effect action",
                    )?;
                    let key = (action.owner.clone(), action.name.clone());
                    if let Some(existing) = actions.insert(key, action) {
                        return invalid(
                            symbol.qualified_path.join("."),
                            format!(
                                "duplicate logical action identity `{}.{}`; first declaration has stable id {:?}",
                                action.owner, action.name, existing.stable_id
                            ),
                        );
                    }
                }
                _ => {}
            }
        }
        Ok(Self { effects, actions })
    }

    fn effect(&self, name: &str) -> Option<&'a crate::EffectDecl> {
        self.effects.get(name).copied()
    }

    fn action(&self, owner: &str, name: &str) -> Option<&'a crate::EffectActionDecl> {
        self.actions
            .get(&(owner.to_owned(), name.to_owned()))
            .copied()
    }
}

fn validate_decl_identity(
    symbol: &crate::StdSymbol,
    expected_kind: StdSymbolKind,
    declaration_name: &str,
    declaration_kind: &str,
) -> Result<(), StdRegistryValidationError> {
    let path = symbol.qualified_path.join(".");
    if symbol.kind != expected_kind {
        return invalid(
            path,
            format!("{declaration_kind} symbol has inconsistent symbol kind"),
        );
    }
    if symbol.name != declaration_name
        || symbol.qualified_path.last().map(String::as_str) != Some(declaration_name)
    {
        return invalid(
            path,
            format!(
                "{declaration_kind} declaration name `{declaration_name}` does not match its symbol identity"
            ),
        );
    }
    Ok(())
}

fn validate_stable_ids(registry: &StdRegistry) -> Result<(), StdRegistryValidationError> {
    let mut effect_ids = BTreeMap::new();
    let mut action_ids = BTreeMap::new();
    for symbol in registry.symbols() {
        let (kind, id, ids) = match &symbol.decl {
            StdDecl::Effect(effect) => ("effect", effect.stable_id, &mut effect_ids),
            StdDecl::EffectAction(action) => ("action", action.stable_id, &mut action_ids),
            _ => continue,
        };
        let Some(id) = id else {
            continue;
        };
        if let Some(existing) = ids.insert(id, symbol.qualified_path.join(".")) {
            return invalid(
                symbol.qualified_path.join("."),
                format!("duplicate {kind} stable id {id}; already used by `{existing}`"),
            );
        }
    }
    Ok(())
}

fn validate_effect_extensions(
    logical_effects: &LogicalEffectIndex<'_>,
    path: &[String],
    effect: &crate::EffectDecl,
) -> Result<(), StdRegistryValidationError> {
    validate_unique_names(path, "extended effect", &effect.extends)?;
    for parent in &effect.extends {
        if parent == &effect.name {
            return invalid(path.join("."), "effect cannot extend itself");
        }
        let declaration =
            logical_effects
                .effect(parent)
                .ok_or_else(|| StdRegistryValidationError {
                    symbol: path.join("."),
                    reason: format!("extended effect `{parent}` is missing"),
                })?;
        if !declaration.params.is_empty() {
            return invalid(
                path.join("."),
                format!(
                    "extended effect `{parent}` requires {} static argument(s), but effect extends does not provide arguments",
                    declaration.params.len()
                ),
            );
        }
    }
    Ok(())
}

fn validate_effect_extension_cycles(
    logical_effects: &LogicalEffectIndex<'_>,
) -> Result<(), StdRegistryValidationError> {
    fn visit(
        logical_effects: &LogicalEffectIndex<'_>,
        name: &str,
        visiting: &mut BTreeSet<String>,
        visited: &mut BTreeSet<String>,
    ) -> Result<(), StdRegistryValidationError> {
        if visited.contains(name) {
            return Ok(());
        }
        if !visiting.insert(name.to_owned()) {
            return invalid(name.to_owned(), "cyclic effect extension graph");
        }
        let effect = logical_effects
            .effect(name)
            .ok_or_else(|| StdRegistryValidationError {
                symbol: name.to_owned(),
                reason: "effect declaration is missing".to_owned(),
            })?;
        for parent in &effect.extends {
            visit(logical_effects, parent, visiting, visited)?;
        }
        visiting.remove(name);
        visited.insert(name.to_owned());
        Ok(())
    }

    let names = logical_effects.effects.keys().cloned().collect::<Vec<_>>();
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for name in names {
        visit(logical_effects, &name, &mut visiting, &mut visited)?;
    }
    Ok(())
}

fn validate_action_owner(
    logical_effects: &LogicalEffectIndex<'_>,
    path: &[String],
    action: &crate::EffectActionDecl,
) -> Result<(), StdRegistryValidationError> {
    if logical_effects.effect(&action.owner).is_none() {
        return invalid(
            path.join("."),
            format!("action owner effect `{}` is missing", action.owner),
        );
    }
    Ok(())
}

fn validate_tool_requirements(
    registry: &StdRegistry,
    path: &[String],
    requirements: &[String],
) -> Result<(), StdRegistryValidationError> {
    validate_unique_names(path, "tool requirement", requirements)?;
    for requirement in requirements {
        let mut matches = registry.symbols().filter(|symbol| {
            matches!(symbol.decl, StdDecl::Requirement(_))
                && (symbol.qualified_path.join(".") == *requirement
                    || (!requirement.contains('.') && symbol.name == *requirement))
        });
        if matches.next().is_none() || matches.next().is_some() {
            return invalid(
                path.join("."),
                format!("tool requirement `{requirement}` is missing or ambiguous"),
            );
        }
    }
    Ok(())
}

fn validate_requirement(
    registry: &StdRegistry,
    path: &[String],
    requirement: &crate::RequirementDecl,
) -> Result<(), StdRegistryValidationError> {
    if requirement.name.is_empty() {
        return invalid(path.join("."), "requirement name must not be empty");
    }
    for ty in &requirement.params {
        validate_type_expr(registry, &path.join("."), ty, &BTreeSet::new())?;
    }
    match (requirement.kind, requirement.semantics) {
        (RequirementKind::Limit, RequirementSemantics::Limit(_))
        | (
            RequirementKind::Policy
            | RequirementKind::Schema
            | RequirementKind::Evidence
            | RequirementKind::Predicate,
            RequirementSemantics::None,
        ) => Ok(()),
        (RequirementKind::Limit, RequirementSemantics::None) => invalid(
            path.join("."),
            "limit requirement is missing limit semantics",
        ),
        (_, RequirementSemantics::Limit(_)) => invalid(
            path.join("."),
            "non-limit requirement cannot declare limit semantics",
        ),
    }
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
        if registry
            .lookup_qualified(&symbol.qualified_path)
            .map(|found| found.id)
            != Some(symbol.id)
        {
            return invalid(
                symbol.qualified_path.join("."),
                "standard declaration conflicts with an existing re-export",
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
        StdType::Function {
            input,
            output,
            effects,
        } => {
            for ty in input.iter().chain(std::iter::once(output.as_ref())) {
                validate_type_expr(registry, owner, ty, generics)?;
            }
            validate_effect_row(
                registry,
                &LogicalEffectIndex::build(registry)?,
                &owner.split('.').map(str::to_owned).collect::<Vec<_>>(),
                effects,
                EffectReferenceKind::Effect,
                generics,
            )
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
    logical_effects: &LogicalEffectIndex<'_>,
    owner: &[String],
    effects: &[StdEffectRef],
    kind: EffectReferenceKind,
    generics: &BTreeSet<&str>,
) -> Result<(), StdRegistryValidationError> {
    for effect in effects {
        validate_effect_ref(registry, logical_effects, owner, effect, kind, generics)?;
    }
    Ok(())
}

fn validate_effect_ref(
    registry: &StdRegistry,
    logical_effects: &LogicalEffectIndex<'_>,
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
            let Some(declaration) = logical_effects.effect(name) else {
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
            let declaration = match effect.path.as_slice() {
                [action_owner, action_name] => logical_effects.action(action_owner, action_name),
                path => registry
                    .lookup_qualified(path)
                    .and_then(|symbol| match &symbol.decl {
                        StdDecl::EffectAction(action) => Some(action),
                        _ => None,
                    }),
            };
            let Some(declaration) = declaration else {
                return invalid(
                    owner_name,
                    format!("unknown action `{}`", effect.path.join(".")),
                );
            };
            let action_owner = &declaration.owner;
            let action_name = &declaration.name;
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
