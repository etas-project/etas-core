use serde::{Deserialize, Serialize};

/// A primitive's reference to an ordinary, source-owned error enum. The host
/// requires these unit variants; it does not define the enum's other variants,
/// exports, declaration identity or language semantics.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, etas_core::RetainedSize)]
pub struct StdNativeEnumReference {
    pub path: Vec<String>,
    pub required_unit_variants: Vec<String>,
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
