use std::collections::BTreeSet;

use super::{
    CodeFingerprint, DefinitionKey, IdentityError, PackageKey, canonical::CanonicalEncoder,
    definition::validate_path,
};

/// Portable declaration lookup bindings. Paths are the package's import surface;
/// identities retain the original owner across aliases and import-root rewrites.
#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
#[serde(deny_unknown_fields)]
pub struct PackageDefinitionOrigins {
    pub package: PackageKey,
    pub modules: Vec<DefinitionModuleOrigin>,
    pub declarations: Vec<DefinitionBindingOrigin>,
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
#[serde(deny_unknown_fields)]
pub struct DefinitionModuleOrigin {
    pub path: Vec<String>,
    pub canonical_path: Vec<String>,
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
#[serde(deny_unknown_fields)]
pub struct DefinitionBindingOrigin {
    pub module: Vec<String>,
    pub declaration: Vec<String>,
    pub definition: DefinitionKey,
}

impl DefinitionBindingOrigin {
    pub fn path(&self) -> Vec<String> {
        self.module
            .iter()
            .chain(&self.declaration)
            .cloned()
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DefinitionOriginsError {
    Identity(IdentityError),
    DuplicateModule(Vec<String>),
    DuplicateDeclaration(Vec<String>),
    MissingModule(Vec<String>),
}

impl std::fmt::Display for DefinitionOriginsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid definition origins: {self:?}")
    }
}
impl std::error::Error for DefinitionOriginsError {}

impl PackageDefinitionOrigins {
    pub fn validate(&self) -> Result<(), DefinitionOriginsError> {
        let mut modules = BTreeSet::new();
        for module in &self.modules {
            validate_path(&module.path, "definition module binding")
                .map_err(DefinitionOriginsError::Identity)?;
            validate_path(&module.canonical_path, "canonical definition module")
                .map_err(DefinitionOriginsError::Identity)?;
            if !modules.insert(&module.path) {
                return Err(DefinitionOriginsError::DuplicateModule(module.path.clone()));
            }
        }
        let mut declarations = BTreeSet::new();
        for binding in &self.declarations {
            if !modules.contains(&binding.module) {
                return Err(DefinitionOriginsError::MissingModule(
                    binding.module.clone(),
                ));
            }
            if binding.declaration.is_empty() {
                return Err(DefinitionOriginsError::Identity(
                    IdentityError::EmptyDeclaration,
                ));
            }
            validate_path(&binding.declaration, "definition lookup declaration")
                .map_err(DefinitionOriginsError::Identity)?;
            let path = binding.path();
            if !declarations.insert(path.clone()) {
                return Err(DefinitionOriginsError::DuplicateDeclaration(path));
            }
        }
        Ok(())
    }

    /// Content identity is descriptive. Trust boundaries still call validate.
    pub fn fingerprint(&self) -> CodeFingerprint {
        let mut encoder = CanonicalEncoder::new("etas.code.package-definition-origins.v1");
        encoder.fingerprint(self.package.fingerprint());
        let mut modules = self.modules.iter().collect::<Vec<_>>();
        modules.sort_by(|a, b| a.path.cmp(&b.path));
        encoder.number(modules.len() as u64);
        for module in modules {
            encoder.path(&module.path);
            encoder.path(&module.canonical_path);
        }
        let mut declarations = self.declarations.iter().collect::<Vec<_>>();
        declarations.sort_by(|a, b| (&a.module, &a.declaration).cmp(&(&b.module, &b.declaration)));
        encoder.number(declarations.len() as u64);
        for declaration in declarations {
            encoder.path(&declaration.module);
            encoder.path(&declaration.declaration);
            encoder.fingerprint(declaration.definition.fingerprint());
        }
        encoder.finish()
    }
}
