use crate::{
    CodeTypeRole, RuntimeTypeRole, StdGenericParam, StdGenericParamKind, StdPrimitiveType,
    StdSymbolKind, TypeDecl, TypeDeclKind,
};

/// Compiler identities and minimum execution ABI for declarations owned by std
/// source. Parameter names, public exports and bodies are not part of this ABI.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub enum StdSourceRole {
    Primitive(StdPrimitiveType),
    Limit,
    Money,
    Index,
    Region,
    WorkspacePath,
    Option,
    Some,
    None,
    Result,
    Ok,
    Err,
    Task,
    Scheduler,
    Continuation,
    Array,
    List,
    Map,
    Set,
    Range,
    Slice,
    Iter,
    CompilerActionSummary,
    CompilerAgentId,
    CompilerCallableId,
    CompilerDefinitionId,
    CompilerDescriptorDetail,
    CompilerDeterminism,
    CompilerEffectRowInfo,
    CompilerGenericOwnerInfo,
    CompilerStaticValueInfo,
    CompilerToolRef,
    CompilerTraceConstraintInfo,
    CompilerTraceSummary,
    CompilerTraceWitness,
    CompilerTypeInfo,
    CompilerAgentDef,
    CompilerCallableDef,
    CompilerSyntaxBlock,
    EffectAgentic,
    EffectNetwork,
    EffectFileIO,
    EffectCommand,
    EffectMemory,
    EffectSecret,
    EffectTime,
    EffectHuman,
    EffectError,
    EffectFs,
    EffectAsync,
}

impl StdSourceRole {
    pub const ALL: [Self; 69] = [
        Self::Money,
        Self::Limit,
        Self::Primitive(StdPrimitiveType::Bool),
        Self::Primitive(StdPrimitiveType::Bytes),
        Self::Primitive(StdPrimitiveType::Char),
        Self::Primitive(StdPrimitiveType::F32),
        Self::Primitive(StdPrimitiveType::F64),
        Self::Primitive(StdPrimitiveType::I128),
        Self::Primitive(StdPrimitiveType::I16),
        Self::Primitive(StdPrimitiveType::I32),
        Self::Primitive(StdPrimitiveType::I64),
        Self::Primitive(StdPrimitiveType::I8),
        Self::Primitive(StdPrimitiveType::ISize),
        Self::Primitive(StdPrimitiveType::Never),
        Self::Primitive(StdPrimitiveType::String),
        Self::Primitive(StdPrimitiveType::U128),
        Self::Primitive(StdPrimitiveType::U16),
        Self::Primitive(StdPrimitiveType::U32),
        Self::Primitive(StdPrimitiveType::U64),
        Self::Primitive(StdPrimitiveType::U8),
        Self::Primitive(StdPrimitiveType::Unit),
        Self::Primitive(StdPrimitiveType::USize),
        Self::Index,
        Self::Region,
        Self::WorkspacePath,
        Self::Option,
        Self::Some,
        Self::None,
        Self::Result,
        Self::Ok,
        Self::Err,
        Self::Task,
        Self::Scheduler,
        Self::Continuation,
        Self::Array,
        Self::List,
        Self::Map,
        Self::Set,
        Self::Range,
        Self::Slice,
        Self::Iter,
        Self::CompilerActionSummary,
        Self::CompilerAgentId,
        Self::CompilerCallableId,
        Self::CompilerDefinitionId,
        Self::CompilerDescriptorDetail,
        Self::CompilerDeterminism,
        Self::CompilerEffectRowInfo,
        Self::CompilerGenericOwnerInfo,
        Self::CompilerStaticValueInfo,
        Self::CompilerToolRef,
        Self::CompilerTraceConstraintInfo,
        Self::CompilerTraceSummary,
        Self::CompilerTraceWitness,
        Self::CompilerTypeInfo,
        Self::CompilerAgentDef,
        Self::CompilerCallableDef,
        Self::CompilerSyntaxBlock,
        Self::EffectAgentic,
        Self::EffectNetwork,
        Self::EffectFileIO,
        Self::EffectCommand,
        Self::EffectMemory,
        Self::EffectSecret,
        Self::EffectTime,
        Self::EffectHuman,
        Self::EffectError,
        Self::EffectFs,
        Self::EffectAsync,
    ];

    pub fn path(self) -> &'static str {
        match self {
            Self::Primitive(StdPrimitiveType::Bool) => "std.core.bool",
            Self::Primitive(StdPrimitiveType::Bytes) => "std.core.bytes",
            Self::Primitive(StdPrimitiveType::Char) => "std.core.char",
            Self::Primitive(StdPrimitiveType::F32) => "std.core.f32",
            Self::Primitive(StdPrimitiveType::F64) => "std.core.f64",
            Self::Primitive(StdPrimitiveType::I128) => "std.core.i128",
            Self::Primitive(StdPrimitiveType::I16) => "std.core.i16",
            Self::Primitive(StdPrimitiveType::I32) => "std.core.i32",
            Self::Primitive(StdPrimitiveType::I64) => "std.core.i64",
            Self::Primitive(StdPrimitiveType::I8) => "std.core.i8",
            Self::Primitive(StdPrimitiveType::ISize) => "std.core.isize",
            Self::Primitive(StdPrimitiveType::Never) => "std.core.never",
            Self::Primitive(StdPrimitiveType::String) => "std.core.string",
            Self::Primitive(StdPrimitiveType::U128) => "std.core.u128",
            Self::Primitive(StdPrimitiveType::U16) => "std.core.u16",
            Self::Primitive(StdPrimitiveType::U32) => "std.core.u32",
            Self::Primitive(StdPrimitiveType::U64) => "std.core.u64",
            Self::Primitive(StdPrimitiveType::U8) => "std.core.u8",
            Self::Primitive(StdPrimitiveType::Unit) => "std.core.unit",
            Self::Primitive(StdPrimitiveType::USize) => "std.core.usize",
            Self::Limit => "std.runtime.limits.Limit",
            Self::Money => "std.runtime.budget.Money",
            Self::Index => "std.core.Index",
            Self::Region => "std.fs.Region",
            Self::WorkspacePath => "std.fs.WorkspacePath",
            Self::Option => "std.option.Option",
            Self::Some => "std.option.Some",
            Self::None => "std.option.None",
            Self::Result => "std.result.Result",
            Self::Ok => "std.result.Ok",
            Self::Err => "std.result.Err",
            Self::Task => "std.task.Task",
            Self::Scheduler => "std.task.Scheduler",
            Self::Continuation => "std.task.Continuation",
            Self::Array => "std.collections.Array",
            Self::List => "std.collections.List",
            Self::Map => "std.collections.Map",
            Self::Set => "std.collections.Set",
            Self::Range => "std.collections.Range",
            Self::Slice => "std.collections.Slice",
            Self::Iter => "std.collections.Iter",
            Self::CompilerActionSummary => "std.compiler.ActionSummary",
            Self::CompilerAgentId => "std.compiler.AgentId",
            Self::CompilerCallableId => "std.compiler.CallableId",
            Self::CompilerDefinitionId => "std.compiler.DefinitionId",
            Self::CompilerDescriptorDetail => "std.compiler.DescriptorDetail",
            Self::CompilerDeterminism => "std.compiler.Determinism",
            Self::CompilerEffectRowInfo => "std.compiler.EffectRowInfo",
            Self::CompilerGenericOwnerInfo => "std.compiler.GenericOwnerInfo",
            Self::CompilerStaticValueInfo => "std.compiler.StaticValueInfo",
            Self::CompilerToolRef => "std.compiler.ToolRef",
            Self::CompilerTraceConstraintInfo => "std.compiler.TraceConstraintInfo",
            Self::CompilerTraceSummary => "std.compiler.TraceSummary",
            Self::CompilerTraceWitness => "std.compiler.TraceWitness",
            Self::CompilerTypeInfo => "std.compiler.TypeInfo",
            Self::CompilerAgentDef => "std.compiler.AgentDef",
            Self::CompilerCallableDef => "std.compiler.CallableDef",
            Self::CompilerSyntaxBlock => "std.compiler.SyntaxBlock",
            Self::EffectAgentic => "std.effects.Agentic",
            Self::EffectNetwork => "std.effects.Network",
            Self::EffectFileIO => "std.effects.FileIO",
            Self::EffectCommand => "std.effects.Command",
            Self::EffectMemory => "std.effects.Memory",
            Self::EffectSecret => "std.effects.Secret",
            Self::EffectTime => "std.effects.Time",
            Self::EffectHuman => "std.effects.Human",
            Self::EffectError => "std.effects.Error",
            Self::EffectFs => "std.effects.Fs",
            Self::EffectAsync => "std.task.Async",
        }
    }

    pub fn from_path(path: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|role| role.path() == path)
    }

    pub fn parameter_kinds(self) -> &'static [StdGenericParamKind] {
        use StdGenericParamKind::{EffectRow, Type};
        match self {
            Self::Primitive(_)
            | Self::Limit
            | Self::Money
            | Self::Index
            | Self::Region
            | Self::Scheduler
            | Self::CompilerActionSummary
            | Self::CompilerAgentId
            | Self::CompilerCallableId
            | Self::CompilerDefinitionId
            | Self::CompilerDescriptorDetail
            | Self::CompilerDeterminism
            | Self::CompilerEffectRowInfo
            | Self::CompilerGenericOwnerInfo
            | Self::CompilerStaticValueInfo
            | Self::CompilerToolRef
            | Self::CompilerTraceConstraintInfo
            | Self::CompilerTraceSummary
            | Self::CompilerTraceWitness
            | Self::CompilerTypeInfo
            | Self::CompilerSyntaxBlock
            | Self::EffectAgentic
            | Self::EffectNetwork
            | Self::EffectFileIO
            | Self::EffectCommand
            | Self::EffectMemory
            | Self::EffectSecret
            | Self::EffectTime
            | Self::EffectHuman
            | Self::EffectFs
            | Self::EffectAsync => &[],
            Self::WorkspacePath
            | Self::Option
            | Self::Some
            | Self::None
            | Self::Continuation
            | Self::Array
            | Self::List
            | Self::Set
            | Self::Range
            | Self::Slice
            | Self::Iter
            | Self::CompilerAgentDef
            | Self::CompilerCallableDef
            | Self::EffectError => &[Type],
            Self::Result | Self::Ok | Self::Err | Self::Map => &[Type, Type],
            Self::Task => &[Type, EffectRow],
        }
    }

    pub fn is_spec(self) -> bool {
        matches!(self, Self::Index | Self::Region)
    }

    pub fn parameter_bounds(self, index: usize) -> Vec<crate::StdSpecRef> {
        if self == Self::WorkspacePath && index == 0 {
            vec![crate::StdSpecRef::new(&["std", "fs", "Region"])]
        } else {
            Vec::new()
        }
    }

    pub fn runtime_role(self) -> Option<RuntimeTypeRole> {
        match self {
            Self::Limit => Some(RuntimeTypeRole::Limit),
            Self::Money => Some(RuntimeTypeRole::Money),
            Self::WorkspacePath => Some(RuntimeTypeRole::WorkspacePath),
            Self::Task => Some(RuntimeTypeRole::Task),
            Self::Scheduler => Some(RuntimeTypeRole::Scheduler),
            Self::Continuation => Some(RuntimeTypeRole::Continuation),
            _ => None,
        }
    }

    pub fn code_role(self) -> Option<CodeTypeRole> {
        match self {
            Self::CompilerActionSummary => Some(CodeTypeRole::ActionSummary),
            Self::CompilerAgentId => Some(CodeTypeRole::AgentId),
            Self::CompilerCallableId => Some(CodeTypeRole::CallableId),
            Self::CompilerDefinitionId => Some(CodeTypeRole::DefinitionId),
            Self::CompilerDescriptorDetail => Some(CodeTypeRole::DescriptorDetail),
            Self::CompilerDeterminism => Some(CodeTypeRole::Determinism),
            Self::CompilerEffectRowInfo => Some(CodeTypeRole::EffectRowInfo),
            Self::CompilerGenericOwnerInfo => Some(CodeTypeRole::GenericOwnerInfo),
            Self::CompilerStaticValueInfo => Some(CodeTypeRole::StaticValueInfo),
            Self::CompilerToolRef => Some(CodeTypeRole::ToolRef),
            Self::CompilerTraceConstraintInfo => Some(CodeTypeRole::TraceConstraintInfo),
            Self::CompilerTraceSummary => Some(CodeTypeRole::TraceSummary),
            Self::CompilerTraceWitness => Some(CodeTypeRole::TraceWitness),
            Self::CompilerTypeInfo => Some(CodeTypeRole::TypeInfo),
            Self::CompilerAgentDef => Some(CodeTypeRole::AgentDefinition),
            Self::CompilerCallableDef => Some(CodeTypeRole::CallableDefinition),
            Self::CompilerSyntaxBlock => Some(CodeTypeRole::SyntaxBlock),
            _ => None,
        }
    }

    pub fn symbol_kind(self) -> StdSymbolKind {
        match self {
            Self::Some | Self::None | Self::Ok | Self::Err => StdSymbolKind::Constructor,
            role if role.effect_execution().is_some() => StdSymbolKind::Effect,
            _ => StdSymbolKind::Type,
        }
    }

    pub fn is_bodyless_type(self) -> bool {
        self.primitive_type().is_some()
            || (self.code_role().is_some() && !self.is_projected_type())
            || matches!(
                self,
                Self::Money
                    | Self::WorkspacePath
                    | Self::Task
                    | Self::Scheduler
                    | Self::Continuation
                    | Self::Array
                    | Self::List
                    | Self::Map
                    | Self::Set
                    | Self::Range
                    | Self::Slice
                    | Self::Iter
            )
    }

    pub fn primitive_type(self) -> Option<StdPrimitiveType> {
        match self {
            Self::Primitive(primitive) => Some(primitive),
            _ => None,
        }
    }

    pub fn is_projected_type(self) -> bool {
        matches!(
            self,
            Self::CompilerAgentDef | Self::CompilerCallableDef | Self::CompilerSyntaxBlock
        )
    }

    /// Stable execution tags and host capability rules, without public signatures
    /// or the source-owned inheritance graph.
    pub fn effect_execution(self) -> Option<(u32, bool, crate::StdRuntimeRequirement)> {
        use crate::StdRuntimeRequirement as R;
        Some(match self {
            Self::EffectAgentic => (0, true, R::Agentic),
            Self::EffectNetwork => (1, true, R::Network),
            Self::EffectFileIO => (2, true, R::FileIO),
            Self::EffectCommand => (3, true, R::Command),
            Self::EffectMemory => (4, true, R::DurableMemory),
            Self::EffectSecret => (6, true, R::SecretAccess),
            Self::EffectTime => (7, true, R::Time),
            Self::EffectError => (8, true, R::RuntimeHandler),
            Self::EffectHuman => (10, true, R::HostAuthority),
            Self::EffectFs => (25, false, R::FileIO),
            Self::EffectAsync => (28, true, R::TaskScheduler),
            _ => return None,
        })
    }

    pub fn query_declaration(self, parameters: Vec<StdGenericParam>) -> crate::StdDecl {
        if let Some((stable_id, core, runtime_requirement)) = self.effect_execution() {
            return crate::StdDecl::Effect(crate::EffectDecl {
                name: self.path().rsplit('.').next().unwrap().to_owned(),
                params: parameters,
                extends: Vec::new(),
                core,
                stable_id: Some(stable_id),
                runtime_requirement: Some(runtime_requirement),
                high_impact_ack: false,
            });
        }
        crate::StdDecl::Type(self.declaration(parameters))
    }

    /// Make a query view from source-owned parameters and the execution ABI.
    pub fn declaration(self, parameters: Vec<StdGenericParam>) -> TypeDecl {
        let kind = match self {
            Self::Primitive(_) => TypeDeclKind::Primitive,
            role if role.code_role().is_some() => TypeDeclKind::Struct,
            role if role.is_spec() => TypeDeclKind::Spec,
            role if role.is_bodyless_type() => TypeDeclKind::Support,
            _ => TypeDeclKind::Enum,
        };
        let name = self.path().rsplit('.').next().unwrap();
        let declaration = TypeDecl::generic_params(name, parameters, kind);
        if let Some(role) = self.code_role() {
            return declaration.code_role(role).intrinsic_only();
        }
        if self == Self::Iter {
            return declaration.iterable_elements(0).intrinsic_only();
        }
        match self.runtime_role() {
            Some(role @ (RuntimeTypeRole::WorkspacePath | RuntimeTypeRole::Limit)) => {
                declaration.runtime_role(role)
            }
            Some(role) => declaration.runtime_role(role).intrinsic_only(),
            None => declaration,
        }
    }

    pub(crate) fn predeclaration(self) -> crate::StdDecl {
        self.query_declaration(
            self.parameter_kinds()
                .iter()
                .enumerate()
                .map(|(index, kind)| StdGenericParam {
                    name: format!("$role{index}"),
                    kind: *kind,
                    bounds: self.parameter_bounds(index),
                })
                .collect(),
        )
    }
}
