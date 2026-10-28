use crate::{
    StdDecl, StdRegistry, StdRegistryValidationError, StdSymbol, StdSymbolKind, StdType,
    StdValueRole, TypeDeclKind,
};
use std::collections::BTreeSet;

pub(super) fn validate(
    registry: &StdRegistry,
    symbol: &StdSymbol,
    roles: &mut BTreeSet<StdValueRole>,
) -> Result<(), StdRegistryValidationError> {
    let StdDecl::Value(value) = &symbol.decl else {
        return Ok(());
    };
    let Some(role) = value.runtime_role else {
        return Ok(());
    };
    let invalid = || {
        StdRegistryValidationError {symbol: symbol.qualified_path.join("."), reason: "standard runtime value requires a unique role, canonical declaration and opaque support owner".into()}
    };
    let mut expected_path = role
        .type_path()
        .iter()
        .map(|name| (*name).to_owned())
        .collect::<Vec<_>>();
    expected_path.pop();
    expected_path.push(role.value_name().into());
    if !roles.insert(role)
        || symbol.kind != StdSymbolKind::Value
        || symbol.qualified_path != expected_path
        || value.name != role.value_name()
        || symbol.name != value.name
        || symbol.enum_owner.is_some()
        || symbol.intrinsic.is_some()
        || value.ty != StdType::Named(role.type_path().join("."))
    {
        return Err(invalid());
    }
    let Some(owner) = registry.lookup_qualified(role.type_path()) else {
        return Err(invalid());
    };
    let StdDecl::Type(owner) = &owner.decl else {
        return Err(invalid());
    };
    if owner.kind != TypeDeclKind::Support
        || !owner.params.is_empty()
        || owner.representation.is_some()
        || owner.canonical_type.is_some()
        || owner.derivable
        || owner.code_role.is_some()
        || owner.runtime_role.is_some()
    {
        return Err(invalid());
    }
    Ok(())
}
