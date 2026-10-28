use std::{fmt, sync::Arc};

use crate::{CodeFingerprint, DefinitionKey, RevisionInfo};

/// An inert reference to backend-owned compiler evidence. An engine must resolve
/// the exact lease and prepare its own implementation before execution.
pub trait CheckedCodeLease: fmt::Debug + etas_core::memory::RetainedSize + Send + Sync {
    fn identity(&self) -> &DefinitionKey;
    fn revision(&self) -> &RevisionInfo;
    fn snapshot_fingerprint(&self) -> CodeFingerprint;
}

#[derive(Clone, Debug)]
pub struct CheckedCodeRef(Arc<dyn CheckedCodeLease>);

impl CheckedCodeRef {
    pub fn from_backend(lease: Arc<dyn CheckedCodeLease>) -> Self {
        Self(lease)
    }
    pub fn identity(&self) -> &DefinitionKey {
        self.0.identity()
    }
    pub fn revision(&self) -> &RevisionInfo {
        self.0.revision()
    }
    pub fn snapshot_fingerprint(&self) -> CodeFingerprint {
        self.0.snapshot_fingerprint()
    }
    pub fn is_backed_by<T: CheckedCodeLease + 'static>(&self, backing: &Arc<T>) -> bool {
        let backing: Arc<dyn CheckedCodeLease> = backing.clone();
        Arc::ptr_eq(&self.0, &backing)
    }
}

impl etas_core::memory::RetainedSize for CheckedCodeRef {
    fn visit_retained<'a>(&'a self, visitor: &mut etas_core::memory::MemoryVisitor<'a>) {
        visitor.visit_shared_arc(&self.0);
    }
}
