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
    subject_arguments: Vec<CodeFingerprint>,
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
        Ok(Self {
            specs,
            subject_arguments: Vec::new(),
        })
    }

    pub fn with_subject_arguments(mut self, arguments: Vec<CodeFingerprint>) -> Self {
        self.subject_arguments = arguments;
        self
    }

    pub fn fingerprint(&self) -> CodeFingerprint {
        let domain = if self.subject_arguments.is_empty() {
            "etas.code.implementation-header.v1"
        } else {
            "etas.code.implementation-header.v2"
        };
        let mut encoder = CanonicalEncoder::new(domain);
        if !self.subject_arguments.is_empty() {
            encoder.number(self.subject_arguments.len() as u64);
            for argument in &self.subject_arguments {
                encoder.fingerprint(*argument);
            }
        }
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
