use serde::{Deserialize, Serialize};

/// A primitive's reference to an ordinary, source-owned error enum. The host
/// requires only the variants and primitive payloads it produces; it does not
/// define the enum's other variants,
/// exports, declaration identity or language semantics.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, etas_core::RetainedSize)]
pub struct StdNativeEnumReference {
    pub path: Vec<String>,
    pub required_unit_variants: Vec<String>,
    pub required_payload_variants: std::collections::BTreeMap<String, Vec<crate::StdPrimitiveType>>,
}

/// A host payload's ABI import. The checked source owns its opaque declaration;
/// the host only requires a canonical, non-generic value without exposed storage.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, etas_core::RetainedSize)]
pub struct StdNativeOpaqueReference {
    pub path: Vec<String>,
}

impl StdNativeOpaqueReference {
    pub(crate) fn matches_declaration(&self, declaration: &crate::StdDecl) -> bool {
        matches!(declaration, crate::StdDecl::Type(declaration)
            if matches!(declaration.kind, crate::TypeDeclKind::Support | crate::TypeDeclKind::Struct)
                && declaration.params.is_empty()
                && declaration.representation.is_none()
                && declaration.canonical_type.is_none()
                && !declaration.derivable)
    }
}

impl StdNativeEnumReference {
    pub fn required_variants(&self) -> impl Iterator<Item = (&str, &[crate::StdPrimitiveType])> {
        self.required_unit_variants
            .iter()
            .map(|name| (name.as_str(), &[][..]))
            .chain(
                self.required_payload_variants
                    .iter()
                    .map(|(name, fields)| (name.as_str(), fields.as_slice())),
            )
    }

    pub(crate) fn matches_owner(
        &self,
        registry: &super::StdRegistry,
        owner: &super::StdSymbol,
    ) -> bool {
        matches!(&owner.decl, crate::StdDecl::Type(declaration)
            if declaration.kind == crate::TypeDeclKind::Enum && declaration.params.is_empty())
            && self.required_variants().all(|(name, fields)| {
                registry.enum_constructor(owner.id, name).is_some_and(|variant| {
                    matches!(&variant.decl, crate::StdDecl::Flow(flow)
                        if flow.params == fields.iter().copied().map(crate::StdType::Primitive).collect::<Vec<_>>())
                })
            })
    }
}

/// A host record payload import. Source owns the nominal declaration and public
/// exports; the host requires the record fields it actually produces.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, etas_core::RetainedSize)]
pub struct StdNativeRecordReference {
    pub path: Vec<String>,
    pub fields: Vec<crate::StdRecordField>,
}

impl StdNativeRecordReference {
    pub(crate) fn matches_declaration(&self, declaration: &crate::StdDecl) -> bool {
        matches!(declaration, crate::StdDecl::Type(declaration)
            if declaration.kind == crate::TypeDeclKind::Struct
                && declaration.params.is_empty()
                && matches!(&declaration.representation, Some(crate::StdType::Record(fields))
                    if fields.len() == self.fields.len() && self.fields.iter().all(|field| fields.contains(field))))
    }
}
