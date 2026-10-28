use std::{fmt, sync::Arc};

use crate::{
    DefinitionKey, RevisionInfo,
    syntax::{SyntaxBlockRef, SyntaxOperationError},
};

/// An immutable backend definition object, not executable evidence. Consumers
/// must resolve the exact lease through their own backend, never by digest alone.
pub trait AgentDefinitionLease: fmt::Debug + etas_core::memory::RetainedSize + Send + Sync {
    fn identity(&self) -> &DefinitionKey;
    fn revision(&self) -> &RevisionInfo;
}

#[derive(Clone, Debug)]
pub struct AgentDefinitionRef(Arc<dyn AgentDefinitionLease>);

impl AgentDefinitionRef {
    pub fn is_same_object(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
    pub fn from_backend(lease: Arc<dyn AgentDefinitionLease>) -> Self {
        Self(lease)
    }
    pub fn identity(&self) -> &DefinitionKey {
        self.0.identity()
    }
    pub fn revision(&self) -> &RevisionInfo {
        self.0.revision()
    }

    pub fn is_backed_by<T: AgentDefinitionLease + 'static>(&self, backing: &Arc<T>) -> bool {
        let backing: Arc<dyn AgentDefinitionLease> = backing.clone();
        Arc::ptr_eq(&self.0, &backing)
    }
}

impl etas_core::memory::RetainedSize for AgentDefinitionRef {
    fn visit_retained<'a>(&'a self, visitor: &mut etas_core::memory::MemoryVisitor<'a>) {
        visitor.visit_shared_arc(&self.0);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub enum DefinitionResource {
    RetainedObjects,
    RetainedBytes,
}

#[derive(Clone, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub enum DefinitionOperationError {
    ForeignDefinition,
    IncompatibleDefinitions,
    Syntax(SyntaxOperationError),
    ResourceLimit {
        resource: DefinitionResource,
        limit: u64,
    },
    BackendUnavailable {
        reason: String,
    },
}

impl fmt::Display for DefinitionOperationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ForeignDefinition => f.write_str("definition belongs to another backend"),
            Self::IncompatibleDefinitions => {
                f.write_str("diff requires two snapshots of the same agent")
            }
            Self::Syntax(error) => error.fmt(f),
            Self::ResourceLimit { resource, limit } => write!(
                f,
                "definition operation exceeded {resource:?} limit {limit}"
            ),
            Self::BackendUnavailable { reason } => {
                write!(f, "definition backend unavailable: {reason}")
            }
        }
    }
}
impl std::error::Error for DefinitionOperationError {}

/// Pure immutable definition editing. It performs no checking, publication, I/O
/// or execution, and a returned definition remains code data rather than proof.
pub trait CodeDefinitionOperations: Send + Sync {
    fn with_body(
        &self,
        definition: &AgentDefinitionRef,
        body: SyntaxBlockRef,
    ) -> Result<AgentDefinitionRef, DefinitionOperationError>;
}
