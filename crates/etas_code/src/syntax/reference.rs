use std::{fmt, sync::Arc};

use crate::{CodeFingerprint, identity::canonical::CanonicalEncoder};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, etas_core::RetainedSize)]
pub struct SyntaxIdentity {
    source: CodeFingerprint,
    content: CodeFingerprint,
    object: CodeFingerprint,
}

impl SyntaxIdentity {
    pub fn new(source: CodeFingerprint, content: CodeFingerprint) -> Self {
        let mut encoder = CanonicalEncoder::new("etas.code.syntax-object.v1");
        encoder.fingerprint(source);
        encoder.fingerprint(content);
        Self {
            source,
            content,
            object: encoder.finish(),
        }
    }

    pub fn source(self) -> CodeFingerprint {
        self.source
    }

    pub fn content(self) -> CodeFingerprint {
        self.content
    }

    pub fn fingerprint(self) -> CodeFingerprint {
        self.object
    }
}

/// Implemented by a trusted syntax backend holding the native structured object.
/// There is deliberately no deserializer or Any/downcast escape hatch.
pub trait SyntaxObjectLease: fmt::Debug + etas_core::memory::RetainedSize + Send + Sync {
    fn identity(&self) -> SyntaxIdentity;
}

#[derive(Clone, Debug)]
pub struct SyntaxBlockRef {
    identity: SyntaxIdentity,
    lease: Arc<dyn SyntaxObjectLease>,
}

impl etas_core::memory::RetainedSize for SyntaxBlockRef {
    fn visit_retained<'a>(&'a self, visitor: &mut etas_core::memory::MemoryVisitor<'a>) {
        visitor.visit_shared_arc(&self.lease);
    }
}

impl SyntaxBlockRef {
    pub fn from_backend(lease: Arc<dyn SyntaxObjectLease>) -> Self {
        Self {
            identity: lease.identity(),
            lease,
        }
    }

    pub fn identity(&self) -> SyntaxIdentity {
        self.identity
    }

    /// A content digest alone must not recover another backend's retained AST.
    pub fn is_backed_by<T: SyntaxObjectLease + 'static>(&self, backing: &Arc<T>) -> bool {
        let backing: Arc<dyn SyntaxObjectLease> = backing.clone();
        Arc::ptr_eq(&self.lease, &backing)
    }
}
