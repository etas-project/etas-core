use crate::{
    StdDecl, StdRegistry, StdRegistryValidationError, StdSymbol, StdSymbolKind, StdType,
    TypeDeclKind,
};

pub(super) fn validate(
    registry: &StdRegistry,
    symbol: &StdSymbol,
) -> Result<(), StdRegistryValidationError> {
    let Some(owner) = symbol.enum_owner else {
        return Ok(());
    };
    let invalid = |reason: &str| StdRegistryValidationError {
        symbol: symbol.qualified_path.join("."),
        reason: reason.into(),
    };
    let owner = registry
        .symbol(owner)
        .ok_or_else(|| invalid("missing enum constructor owner"))?;
    let StdDecl::Type(decl) = &owner.decl else {
        return Err(invalid("constructor owner is not a type"));
    };
    if decl.kind != TypeDeclKind::Enum || owner.enum_owner.is_some() {
        return Err(invalid("constructor owner must be a module-level enum"));
    }
    let StdDecl::Flow(flow) = &symbol.decl else {
        return Err(invalid("enum constructor must have a callable signature"));
    };
    let result_role = owner.qualified_path == ["std", "result", "Result"];
    let option_role = owner.qualified_path == ["std", "option", "Option"];
    if result_role && decl.params.len() != 2 {
        return Err(invalid("Result role requires two type parameters"));
    }
    if option_role && decl.params.len() != 1 {
        return Err(invalid("Option role requires one type parameter"));
    }
    let mut expected_path = owner.qualified_path.clone();
    expected_path.push(flow.name.clone());
    if symbol.kind != StdSymbolKind::Constructor
        || symbol.module != owner.module
        || symbol.name != flow.name
        || symbol.qualified_path != expected_path
    {
        return Err(invalid(
            "enum constructor identity does not match its owner and declaration",
        ));
    }
    if flow.type_params != decl.params
        || !flow.public_effects.is_empty()
        || !flow.requested_actions.is_empty()
        || flow.source_method.is_some()
        || symbol.intrinsic.is_some()
    {
        return Err(invalid(
            "enum constructor must preserve owner parameters and have no effects, method, or intrinsic",
        ));
    }
    let name = owner.qualified_path.join(".");
    let output = if result_role {
        StdType::Result {
            ok: Box::new(StdType::Var(decl.params[0].name.clone())),
            err: Box::new(StdType::Var(decl.params[1].name.clone())),
        }
    } else if option_role {
        StdType::Option(Box::new(StdType::Var(decl.params[0].name.clone())))
    } else if decl.params.is_empty() {
        StdType::Named(name)
    } else {
        StdType::NamedApplied {
            name,
            args: decl
                .params
                .iter()
                .map(|param| StdType::Var(param.name.clone()))
                .collect(),
        }
    };
    if flow.output != output {
        return Err(invalid(
            "enum constructor result must be its canonical owner applied to the declared parameters",
        ));
    }
    Ok(())
}
