use crate::{
    RuntimeTypeRole, StdGenericParam, StdGenericParamKind, StdSymbolKind, TypeDecl, TypeDeclKind,
};

/// Compiler identities and minimum execution ABI for declarations owned by std
/// source. Parameter names, public exports and bodies are not part of this ABI.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub enum StdSourceRole {
    Index,
    Option,
    Some,
    None,
    Result,
    Ok,
    Err,
    Task,
    Scheduler,
    Continuation,
}

impl StdSourceRole {
    pub const ALL: [Self; 10] = [
        Self::Index,
        Self::Option,
        Self::Some,
        Self::None,
        Self::Result,
        Self::Ok,
        Self::Err,
        Self::Task,
        Self::Scheduler,
        Self::Continuation,
    ];

    pub fn path(self) -> &'static str {
        match self {
            Self::Index => "std.core.Index",
            Self::Option => "std.option.Option",
            Self::Some => "std.option.Some",
            Self::None => "std.option.None",
            Self::Result => "std.result.Result",
            Self::Ok => "std.result.Ok",
            Self::Err => "std.result.Err",
            Self::Task => "std.task.Task",
            Self::Scheduler => "std.task.Scheduler",
            Self::Continuation => "std.task.Continuation",
        }
    }

    pub fn from_path(path: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|role| role.path() == path)
    }

    pub fn parameter_kinds(self) -> &'static [StdGenericParamKind] {
        use StdGenericParamKind::{EffectRow, Type};
        match self {
            Self::Index | Self::Scheduler => &[],
            Self::Option | Self::Some | Self::None | Self::Continuation => &[Type],
            Self::Result | Self::Ok | Self::Err => &[Type, Type],
            Self::Task => &[Type, EffectRow],
        }
    }

    pub fn runtime_role(self) -> Option<RuntimeTypeRole> {
        match self {
            Self::Task => Some(RuntimeTypeRole::Task),
            Self::Scheduler => Some(RuntimeTypeRole::Scheduler),
            Self::Continuation => Some(RuntimeTypeRole::Continuation),
            _ => None,
        }
    }

    pub fn symbol_kind(self) -> StdSymbolKind {
        match self {
            Self::Some | Self::None | Self::Ok | Self::Err => StdSymbolKind::Constructor,
            _ => StdSymbolKind::Type,
        }
    }

    /// Make a query view from source-owned parameters and the execution ABI.
    pub fn declaration(self, parameters: Vec<StdGenericParam>) -> TypeDecl {
        let kind = match self {
            Self::Index => TypeDeclKind::Spec,
            Self::Task | Self::Scheduler | Self::Continuation => TypeDeclKind::Support,
            _ => TypeDeclKind::Enum,
        };
        let name = self.path().rsplit('.').next().unwrap();
        let declaration = TypeDecl::generic_params(name, parameters, kind);
        match self.runtime_role() {
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
                    bounds: Vec::new(),
                })
                .collect(),
        )
    }
}
