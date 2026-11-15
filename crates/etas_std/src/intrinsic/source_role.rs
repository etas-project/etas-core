use crate::{
    CodeTypeRole, RuntimeTypeRole, StdGenericParam, StdGenericParamKind, StdSymbolKind, TypeDecl,
    TypeDeclKind,
};

/// Compiler identities and minimum execution ABI for declarations owned by std
/// source. Parameter names, public exports and bodies are not part of this ABI.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub enum StdSourceRole {
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
}

impl StdSourceRole {
    pub const ALL: [Self; 36] = [
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
    ];

    pub fn path(self) -> &'static str {
        match self {
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
        }
    }

    pub fn from_path(path: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|role| role.path() == path)
    }

    pub fn parameter_kinds(self) -> &'static [StdGenericParamKind] {
        use StdGenericParamKind::{EffectRow, Type};
        match self {
            Self::Index
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
            | Self::CompilerSyntaxBlock => &[],
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
            | Self::CompilerCallableDef => &[Type],
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
            _ => StdSymbolKind::Type,
        }
    }

    pub fn is_bodyless_type(self) -> bool {
        (self.code_role().is_some() && !self.is_projected_type())
            || matches!(
                self,
                Self::WorkspacePath
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

    pub fn is_projected_type(self) -> bool {
        matches!(
            self,
            Self::CompilerAgentDef | Self::CompilerCallableDef | Self::CompilerSyntaxBlock
        )
    }

    /// Make a query view from source-owned parameters and the execution ABI.
    pub fn declaration(self, parameters: Vec<StdGenericParam>) -> TypeDecl {
        let kind = match self {
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
            Some(RuntimeTypeRole::WorkspacePath) => {
                declaration.runtime_role(RuntimeTypeRole::WorkspacePath)
            }
            Some(role) => declaration.runtime_role(role).intrinsic_only(),
            None => declaration,
        }
    }

    pub(crate) fn predeclaration(self) -> TypeDecl {
        self.declaration(
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
