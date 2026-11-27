//! Minimal execution mechanisms whose declarations belong to Etas source.
//! Ordinary standard-library algorithms cannot acquire one of these roles.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    etas_core::RetainedSize,
)]
pub enum StdExecutionRole {
    Abort,
    Checkpoint,
    CaptureData,
    StorageContext,
    StorageNonce,
    StreamClock,
    Requeue,
    Park,
    TaskStart,
    TaskNextCompletion,
    TaskCancel,
    ScopeEnter,
    ParseBody,
    AgentWithBody,
    CallableWithBody,
    CurrentSession,
    ToolReference,
    MessageContextCapture,
    MessageContextRecord,
    PayloadConvert,
    ComparatorCapture,
    ComparatorSame,
    ReadScopeEnter,
    CaptureStorage,
    RestoreStorage,
    StorageSchema,
    ResourceScope,
    CachedSchemaFingerprint,
    RememberSchemaFingerprint,
}

impl StdExecutionRole {
    pub const ALL: [Self; 29] = [
        Self::Abort,
        Self::Checkpoint,
        Self::CaptureData,
        Self::StorageContext,
        Self::StorageNonce,
        Self::StreamClock,
        Self::Requeue,
        Self::Park,
        Self::TaskStart,
        Self::TaskNextCompletion,
        Self::TaskCancel,
        Self::ScopeEnter,
        Self::ParseBody,
        Self::AgentWithBody,
        Self::CallableWithBody,
        Self::CurrentSession,
        Self::ToolReference,
        Self::MessageContextCapture,
        Self::MessageContextRecord,
        Self::PayloadConvert,
        Self::ComparatorCapture,
        Self::ComparatorSame,
        Self::ReadScopeEnter,
        Self::CaptureStorage,
        Self::RestoreStorage,
        Self::StorageSchema,
        Self::ResourceScope,
        Self::CachedSchemaFingerprint,
        Self::RememberSchemaFingerprint,
    ];

    pub fn role(self) -> &'static str {
        match self {
            Self::Abort => "std.core.abort",
            Self::Checkpoint => "std.runtime.checkpoint",
            Self::CaptureData => "std.runtime.value.Data.capture",
            Self::CaptureStorage => "std.memory.StorageValue.capture",
            Self::RestoreStorage => "std.memory.StorageValue.restore",
            Self::StorageSchema => "std.memory.StorageValue.schema",
            Self::CachedSchemaFingerprint => "std.memory.MutationFingerprint.cached_schema",
            Self::RememberSchemaFingerprint => "std.memory.MutationFingerprint.remember_schema",
            Self::StorageContext => "std.memory.StorageContext.current",
            Self::StorageNonce => "std.memory.StorageContext.nonce",
            Self::StreamClock => "std.stream.StreamClock.now",
            Self::Requeue => "std.task.Scheduler.requeue",
            Self::Park => "std.task.Scheduler.park",
            Self::TaskStart => "std.task.start",
            Self::TaskNextCompletion => "std.task.Scheduler.next_completion",
            Self::TaskCancel => "std.task.Scheduler.cancel",
            Self::ScopeEnter => "std.task.Scope.enter",
            Self::ResourceScope => "std.resource.Scope.enter",
            Self::ParseBody => "std.compiler.parse_body",
            Self::AgentWithBody => "std.compiler.AgentDef.with_body",
            Self::CallableWithBody => "std.compiler.CallableDef.with_body",
            Self::CurrentSession => "std.agent.session.current_session",
            Self::ToolReference => "std.tool.reference",
            Self::MessageContextCapture => "std.agent.message.MessageContext.capture",
            Self::MessageContextRecord => "std.agent.message.MessageContext.record",
            Self::PayloadConvert => "std.agent.message.PayloadRepresentation.convert",
            Self::ComparatorCapture => "std.collections.ComparatorIdentity.capture",
            Self::ComparatorSame => "std.collections.ComparatorIdentity.same",
            Self::ReadScopeEnter => "std.stream.ReadScope.enter",
        }
    }

    pub fn source_path(self) -> &'static str {
        match self {
            Self::Abort => "std.core.abort",
            Self::Checkpoint => "std.runtime.checkpoint.checkpoint",
            Self::CaptureData => "std.runtime.value.Data.capture",
            Self::CaptureStorage => "std.memory.StorageValue.capture",
            Self::RestoreStorage => "std.memory.StorageValue.restore",
            Self::StorageSchema => "std.memory.StorageValue.schema",
            Self::CachedSchemaFingerprint => "std.memory.MutationFingerprint.cached_schema",
            Self::RememberSchemaFingerprint => "std.memory.MutationFingerprint.remember_schema",
            Self::StorageContext => "std.memory.StorageContext.current",
            Self::StorageNonce => "std.memory.StorageContext.nonce",
            Self::StreamClock => "std.stream.StreamClock.now",
            Self::Requeue => "std.task.Scheduler.requeue",
            Self::Park => "std.task.Scheduler.park",
            Self::TaskStart => "std.task.start",
            Self::TaskNextCompletion => "std.task.Scheduler.next_completion",
            Self::TaskCancel => "std.task.Scheduler.cancel",
            Self::ScopeEnter => "std.task.Scope.enter",
            Self::ResourceScope => "std.resource.Scope.enter",
            Self::ParseBody => "std.compiler.parse_body",
            Self::AgentWithBody => "std.compiler.AgentDef.with_body",
            Self::CallableWithBody => "std.compiler.CallableDef.with_body",
            Self::CurrentSession => "std.agent.session.current_session",
            Self::ToolReference => "std.tool.reference",
            Self::MessageContextCapture => "std.agent.message.MessageContext.capture",
            Self::MessageContextRecord => "std.agent.message.MessageContext.record",
            Self::PayloadConvert => "std.agent.message.PayloadRepresentation.convert",
            Self::ComparatorCapture => "std.collections.ComparatorIdentity.capture",
            Self::ComparatorSame => "std.collections.ComparatorIdentity.same",
            Self::ReadScopeEnter => "std.stream.ReadScope.enter",
        }
    }

    /// Transitional native contracts exist only for the old free execution flows.
    /// Source-owned associated mechanisms validate their checked source ABI directly.
    pub fn native_contract_path(self) -> Option<&'static str> {
        match self {
            Self::Abort | Self::Checkpoint => Some(self.role()),
            Self::CaptureData
            | Self::CaptureStorage
            | Self::RestoreStorage
            | Self::StorageSchema
            | Self::CachedSchemaFingerprint
            | Self::RememberSchemaFingerprint
            | Self::StorageContext
            | Self::StorageNonce
            | Self::StreamClock
            | Self::Requeue
            | Self::Park
            | Self::TaskStart
            | Self::TaskNextCompletion
            | Self::TaskCancel
            | Self::ScopeEnter
            | Self::ResourceScope
            | Self::ParseBody
            | Self::AgentWithBody
            | Self::CallableWithBody
            | Self::CurrentSession
            | Self::ToolReference
            | Self::MessageContextCapture
            | Self::MessageContextRecord
            | Self::PayloadConvert
            | Self::ComparatorCapture
            | Self::ComparatorSame
            | Self::ReadScopeEnter => None,
        }
    }

    /// Canonical declaring type for the named execution role.
    pub fn associated_owner(self) -> Option<&'static str> {
        self.associated_owners().first().copied()
    }

    pub fn associated_owners(self) -> &'static [&'static str] {
        match self {
            Self::Abort
            | Self::Checkpoint
            | Self::TaskStart
            | Self::ParseBody
            | Self::CurrentSession
            | Self::ToolReference => &[],
            Self::CaptureData => &["std.runtime.value.Data"],
            Self::CaptureStorage | Self::RestoreStorage | Self::StorageSchema => {
                &["std.memory.StorageValue"]
            }
            Self::CachedSchemaFingerprint | Self::RememberSchemaFingerprint => {
                &["std.memory.MutationFingerprint"]
            }
            Self::StorageContext | Self::StorageNonce => &["std.memory.StorageContext"],
            Self::StreamClock => &["std.stream.StreamClock"],
            Self::ReadScopeEnter => &["std.stream.ReadScope"],
            Self::Requeue | Self::Park | Self::TaskNextCompletion | Self::TaskCancel => {
                &["std.task.Scheduler"]
            }
            Self::ScopeEnter => &["std.task.Scope"],
            Self::ResourceScope => &["std.resource.Scope", "std.agent.session.LeaseScope"],
            Self::AgentWithBody => &["std.compiler.AgentDef"],
            Self::CallableWithBody => &["std.compiler.CallableDef"],
            Self::MessageContextCapture | Self::MessageContextRecord => {
                &["std.agent.message.MessageContext"]
            }
            Self::PayloadConvert => &["std.agent.message.PayloadRepresentation"],
            Self::ComparatorCapture | Self::ComparatorSame => {
                &["std.collections.ComparatorIdentity"]
            }
        }
    }

    pub fn is_public(self) -> bool {
        !matches!(
            self,
            Self::StreamClock
                | Self::CaptureStorage
                | Self::RestoreStorage
                | Self::StorageSchema
                | Self::CachedSchemaFingerprint
                | Self::RememberSchemaFingerprint
                | Self::ReadScopeEnter
                | Self::ScopeEnter
                | Self::ResourceScope
                | Self::TaskNextCompletion
                | Self::TaskCancel
                | Self::MessageContextCapture
                | Self::MessageContextRecord
                | Self::PayloadConvert
                | Self::ComparatorCapture
                | Self::ComparatorSame
        )
    }

    pub fn has_receiver(self) -> bool {
        matches!(
            self,
            Self::Requeue
                | Self::Park
                | Self::TaskNextCompletion
                | Self::TaskCancel
                | Self::AgentWithBody
                | Self::CallableWithBody
                | Self::MessageContextRecord
                | Self::ComparatorSame
        )
    }

    pub fn owner_is_public(self) -> bool {
        matches!(
            self,
            Self::CaptureData
                | Self::StorageContext
                | Self::StorageNonce
                | Self::Requeue
                | Self::Park
                | Self::TaskNextCompletion
                | Self::TaskCancel
                | Self::AgentWithBody
                | Self::CallableWithBody
        )
    }

    pub fn owner_type_parameter_count(self) -> usize {
        usize::from(matches!(self, Self::AgentWithBody | Self::CallableWithBody))
    }

    pub fn from_role(role: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.role() == role)
    }

    pub fn accepts_identity(self, callable: &etas_code::CallableId) -> bool {
        let path = |definition: &etas_code::DefinitionKey| {
            definition
                .module()
                .iter()
                .chain(definition.declaration())
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join(".")
        };
        match self.associated_owners() {
            [] => callable.as_declaration().is_some_and(|definition| {
                definition.kind() == etas_code::DefinitionKind::Flow
                    && path(definition) == self.source_path()
            }),
            owners => callable.as_method().is_some_and(|method| {
                method.implementation().selected_spec().is_none()
                    && owners.contains(&path(method.implementation().subject()).as_str())
                    && self.source_path().rsplit('.').next() == Some(method.name())
            }),
        }
    }
}
