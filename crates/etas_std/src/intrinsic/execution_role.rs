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
}

impl StdExecutionRole {
    pub const ALL: [Self; 20] = [
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
    ];

    pub fn role(self) -> &'static str {
        match self {
            Self::Abort => "std.core.abort",
            Self::Checkpoint => "std.runtime.checkpoint",
            Self::CaptureData => "std.runtime.value.Data.capture",
            Self::StorageContext => "std.memory.StorageContext.current",
            Self::StorageNonce => "std.memory.StorageContext.nonce",
            Self::StreamClock => "std.stream.StreamClock.now",
            Self::Requeue => "std.task.Scheduler.requeue",
            Self::Park => "std.task.Scheduler.park",
            Self::TaskStart => "std.task.start",
            Self::TaskNextCompletion => "std.task.Scheduler.next_completion",
            Self::TaskCancel => "std.task.Scheduler.cancel",
            Self::ScopeEnter => "std.task.Scope.enter",
            Self::ParseBody => "std.compiler.parse_body",
            Self::AgentWithBody => "std.compiler.AgentDef.with_body",
            Self::CallableWithBody => "std.compiler.CallableDef.with_body",
            Self::CurrentSession => "std.agent.session.current_session",
            Self::ToolReference => "std.tool.reference",
            Self::MessageContextCapture => "std.agent.message.MessageContext.capture",
            Self::MessageContextRecord => "std.agent.message.MessageContext.record",
            Self::PayloadConvert => "std.agent.message.PayloadRepresentation.convert",
        }
    }

    pub fn source_path(self) -> &'static str {
        match self {
            Self::Abort => "std.core.abort",
            Self::Checkpoint => "std.runtime.checkpoint.checkpoint",
            Self::CaptureData => "std.runtime.value.Data.capture",
            Self::StorageContext => "std.memory.StorageContext.current",
            Self::StorageNonce => "std.memory.StorageContext.nonce",
            Self::StreamClock => "std.stream.StreamClock.now",
            Self::Requeue => "std.task.Scheduler.requeue",
            Self::Park => "std.task.Scheduler.park",
            Self::TaskStart => "std.task.start",
            Self::TaskNextCompletion => "std.task.Scheduler.next_completion",
            Self::TaskCancel => "std.task.Scheduler.cancel",
            Self::ScopeEnter => "std.task.Scope.enter",
            Self::ParseBody => "std.compiler.parse_body",
            Self::AgentWithBody => "std.compiler.AgentDef.with_body",
            Self::CallableWithBody => "std.compiler.CallableDef.with_body",
            Self::CurrentSession => "std.agent.session.current_session",
            Self::ToolReference => "std.tool.reference",
            Self::MessageContextCapture => "std.agent.message.MessageContext.capture",
            Self::MessageContextRecord => "std.agent.message.MessageContext.record",
            Self::PayloadConvert => "std.agent.message.PayloadRepresentation.convert",
        }
    }

    /// Transitional native contracts exist only for the old free execution flows.
    /// Source-owned associated mechanisms validate their checked source ABI directly.
    pub fn native_contract_path(self) -> Option<&'static str> {
        match self {
            Self::Abort | Self::Checkpoint => Some(self.role()),
            Self::CaptureData
            | Self::StorageContext
            | Self::StorageNonce
            | Self::StreamClock
            | Self::Requeue
            | Self::Park
            | Self::TaskStart
            | Self::TaskNextCompletion
            | Self::TaskCancel
            | Self::ScopeEnter
            | Self::ParseBody
            | Self::AgentWithBody
            | Self::CallableWithBody
            | Self::CurrentSession
            | Self::ToolReference
            | Self::MessageContextCapture
            | Self::MessageContextRecord
            | Self::PayloadConvert => None,
        }
    }

    pub fn associated_owner(self) -> Option<&'static str> {
        match self {
            Self::Abort
            | Self::Checkpoint
            | Self::TaskStart
            | Self::ParseBody
            | Self::CurrentSession
            | Self::ToolReference => None,
            Self::CaptureData => Some("std.runtime.value.Data"),
            Self::StorageContext | Self::StorageNonce => Some("std.memory.StorageContext"),
            Self::StreamClock => Some("std.stream.StreamClock"),
            Self::Requeue | Self::Park | Self::TaskNextCompletion | Self::TaskCancel => {
                Some("std.task.Scheduler")
            }
            Self::ScopeEnter => Some("std.task.Scope"),
            Self::AgentWithBody => Some("std.compiler.AgentDef"),
            Self::CallableWithBody => Some("std.compiler.CallableDef"),
            Self::MessageContextCapture | Self::MessageContextRecord => {
                Some("std.agent.message.MessageContext")
            }
            Self::PayloadConvert => Some("std.agent.message.PayloadRepresentation"),
        }
    }

    pub fn is_public(self) -> bool {
        !matches!(
            self,
            Self::StreamClock
                | Self::ScopeEnter
                | Self::TaskNextCompletion
                | Self::TaskCancel
                | Self::MessageContextCapture
                | Self::MessageContextRecord
                | Self::PayloadConvert
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
        match self.associated_owner() {
            None => callable.as_declaration().is_some_and(|definition| {
                definition.kind() == etas_code::DefinitionKind::Flow
                    && path(definition) == self.source_path()
            }),
            Some(owner) => callable.as_method().is_some_and(|method| {
                method.implementation().selected_spec().is_none()
                    && path(method.implementation().subject()) == owner
                    && self.source_path().rsplit('.').next() == Some(method.name())
            }),
        }
    }
}
