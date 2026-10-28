use super::{
    CodeFingerprint, DefinitionKey, DefinitionKind, IdentityError, ImplementationKey, PackageKey,
    canonical::CanonicalEncoder,
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, etas_core::RetainedSize)]
pub struct ImplementationSpecIdentity {
    pub definition: DefinitionKey,
    /// Fingerprints of resolved, explicitly declared header arguments. Inferred
    /// method contracts remain separate from declaration/implementation identity.
    pub arguments: Vec<CodeFingerprint>,
}

#[derive(Clone, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub struct ImplementationHeader {
    specs: Vec<ImplementationSpecIdentity>,
}

impl ImplementationHeader {
    pub fn new(mut specs: Vec<ImplementationSpecIdentity>) -> Result<Self, IdentityError> {
        for spec in &specs {
            if spec.definition.kind() != DefinitionKind::Spec {
                return Err(IdentityError::NotSpec(spec.definition.kind()));
            }
        }
        specs.sort();
        specs.dedup();
        Ok(Self { specs })
    }

    pub fn fingerprint(&self) -> CodeFingerprint {
        let mut encoder = CanonicalEncoder::new("etas.code.implementation-header.v1");
        encoder.number(self.specs.len() as u64);
        for spec in &self.specs {
            encoder.fingerprint(spec.definition.fingerprint());
            encoder.number(spec.arguments.len() as u64);
            for argument in &spec.arguments {
                encoder.fingerprint(*argument);
            }
        }
        encoder.finish()
    }
}

impl ImplementationKey {
    pub fn from_header(
        package: PackageKey,
        module: Vec<String>,
        subject: DefinitionKey,
        selected_spec: Option<DefinitionKey>,
        header: &ImplementationHeader,
    ) -> Result<Self, IdentityError> {
        Self::new(
            package,
            module,
            subject,
            selected_spec,
            header.fingerprint(),
        )
    }
}
