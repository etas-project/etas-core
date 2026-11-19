use super::std_type::StdType;

#[derive(
    serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq, etas_core::RetainedSize,
)]
pub struct TypeDecl {
    pub name: String,
    pub params: Vec<StdGenericParam>,
    pub kind: TypeDeclKind,
    pub representation: Option<StdType>,
    /// Structural compiler support template or checked source alias target;
    /// never nominal storage.
    pub canonical_type: Option<StdType>,
    pub derivable: bool,
    pub iterable_item: Option<StdIterableItem>,
    pub code_role: Option<CodeTypeRole>,
    pub runtime_role: Option<super::RuntimeTypeRole>,
    pub construction: StdTypeConstruction,
    /// Read-only projections, not a constructor representation or Host decode schema.
    pub projected_fields: Vec<super::std_type::StdRecordField>,
}

impl TypeDecl {
    pub fn primitive(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            params: Vec::new(),
            kind: TypeDeclKind::Primitive,
            representation: None,
            canonical_type: None,
            derivable: false,
            iterable_item: None,
            code_role: None,
            runtime_role: None,
            construction: StdTypeConstruction::Public,
            projected_fields: Vec::new(),
        }
    }

    pub fn generic(name: &str, params: &[&str], kind: TypeDeclKind) -> Self {
        Self::generic_params(
            name,
            params
                .iter()
                .map(|name| StdGenericParam::new(name))
                .collect(),
            kind,
        )
    }

    pub fn generic_params(name: &str, params: Vec<StdGenericParam>, kind: TypeDeclKind) -> Self {
        Self {
            name: name.to_owned(),
            params,
            kind,
            representation: None,
            canonical_type: None,
            derivable: false,
            iterable_item: None,
            code_role: None,
            runtime_role: None,
            construction: StdTypeConstruction::Public,
            projected_fields: Vec::new(),
        }
    }

    pub fn with_representation(mut self, representation: StdType) -> Self {
        self.representation = Some(representation);
        self
    }

    pub fn with_canonical_type(mut self, ty: StdType) -> Self {
        self.canonical_type = Some(ty);
        self
    }

    pub fn derivable(mut self) -> Self {
        self.derivable = true;
        self
    }

    pub fn iterable_elements(mut self, parameter: usize) -> Self {
        self.iterable_item = Some(StdIterableItem::Element { parameter });
        self
    }

    pub fn iterable_entries(mut self, key: usize, value: usize) -> Self {
        self.iterable_item = Some(StdIterableItem::Entry { key, value });
        self
    }

    pub fn code_role(mut self, role: CodeTypeRole) -> Self {
        self.code_role = Some(role);
        self
    }
    pub fn intrinsic_only(mut self) -> Self {
        self.construction = StdTypeConstruction::IntrinsicOnly;
        self
    }

    pub fn runtime_role(mut self, role: super::RuntimeTypeRole) -> Self {
        self.runtime_role = Some(role);
        if !role.fields().is_empty() {
            self.projected_fields = role.projected_fields();
        }
        self
    }

    pub fn with_projected_fields(mut self, fields: Vec<super::std_type::StdRecordField>) -> Self {
        self.projected_fields = fields;
        self
    }
}

#[derive(
    serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq, etas_core::RetainedSize,
)]
pub enum StdIterableItem {
    Element { parameter: usize },
    Entry { key: usize, value: usize },
}

impl StdIterableItem {
    pub fn parameters(self) -> impl Iterator<Item = usize> {
        let (first, second) = match self {
            Self::Element { parameter } => (parameter, None),
            Self::Entry { key, value } => (key, Some(value)),
        };
        std::iter::once(first).chain(second)
    }
}

#[derive(
    serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq, etas_core::RetainedSize,
)]
pub enum StdTypeConstruction {
    Public,
    IntrinsicOnly,
}

#[derive(
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    etas_core::RetainedSize,
)]
pub enum CodeTypeRole {
    AgentDefinition,
    CallableDefinition,
    SyntaxBlock,
    AgentId,
    TypeInfo,
    GenericOwnerInfo,
    ToolRef,
    StaticValueInfo,
    TraceConstraintInfo,
    DefinitionId,
    CallableId,
    EffectRowInfo,
    ActionSummary,
    TraceSummary,
    Determinism,
    DescriptorDetail,
    TraceWitness,
}

impl CodeTypeRole {
    /// Compiler observations associated with source-owned data declarations.
    /// This table contains identities only; shapes and generic binders come
    /// from the selected std source bundle.
    pub const ALL: [Self; 17] = [
        Self::AgentDefinition,
        Self::CallableDefinition,
        Self::SyntaxBlock,
        Self::AgentId,
        Self::TypeInfo,
        Self::GenericOwnerInfo,
        Self::ToolRef,
        Self::StaticValueInfo,
        Self::TraceConstraintInfo,
        Self::DefinitionId,
        Self::CallableId,
        Self::EffectRowInfo,
        Self::ActionSummary,
        Self::TraceSummary,
        Self::Determinism,
        Self::DescriptorDetail,
        Self::TraceWitness,
    ];

    pub fn source_path(self) -> &'static str {
        match self {
            Self::AgentDefinition => "std.compiler.AgentDef",
            Self::CallableDefinition => "std.compiler.CallableDef",
            Self::SyntaxBlock => "std.compiler.SyntaxBlock",
            Self::AgentId => "std.compiler.AgentId",
            Self::TypeInfo => "std.compiler.TypeInfo",
            Self::GenericOwnerInfo => "std.compiler.GenericOwnerInfo",
            Self::ToolRef => "std.agent.ToolRef",
            Self::StaticValueInfo => "std.compiler.StaticValueInfo",
            Self::TraceConstraintInfo => "std.compiler.TraceConstraintInfo",
            Self::DefinitionId => "std.compiler.DefinitionId",
            Self::CallableId => "std.compiler.CallableId",
            Self::EffectRowInfo => "std.compiler.EffectRowInfo",
            Self::ActionSummary => "std.compiler.ActionSummary",
            Self::TraceSummary => "std.compiler.TraceSummary",
            Self::Determinism => "std.compiler.Determinism",
            Self::DescriptorDetail => "std.compiler.DescriptorDetail",
            Self::TraceWitness => "std.compiler.TraceWitness",
        }
    }

    pub fn from_source_path(path: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|role| role.source_path() == path)
    }

    pub fn type_parameter_count(self) -> usize {
        usize::from(matches!(
            self,
            Self::AgentDefinition | Self::CallableDefinition
        ))
    }

    pub fn has_read_only_erased_view(self) -> bool {
        self == Self::CallableDefinition
    }
}

#[derive(
    serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq, etas_core::RetainedSize,
)]
pub struct StdGenericParam {
    pub name: String,
    pub kind: StdGenericParamKind,
    pub bounds: Vec<StdSpecRef>,
}

#[derive(
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    etas_core::RetainedSize,
)]
pub enum StdGenericParamKind {
    #[default]
    Type,
    EffectRow,
}

impl StdGenericParam {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            kind: StdGenericParamKind::Type,
            bounds: Vec::new(),
        }
    }

    pub fn effect_row(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            kind: StdGenericParamKind::EffectRow,
            bounds: Vec::new(),
        }
    }

    pub fn bounded(name: &str, bounds: &[StdSpecRef]) -> Self {
        Self {
            name: name.to_owned(),
            kind: StdGenericParamKind::Type,
            bounds: bounds.to_vec(),
        }
    }
}

#[derive(
    serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq, etas_core::RetainedSize,
)]
pub struct StdSpecRef {
    pub path: Vec<String>,
    pub args: Vec<StdType>,
}

impl StdSpecRef {
    pub fn new(path: &[&str]) -> Self {
        Self {
            path: path.iter().map(|segment| (*segment).to_owned()).collect(),
            args: Vec::new(),
        }
    }

    pub fn with_args(path: &[&str], args: Vec<StdType>) -> Self {
        Self {
            path: path.iter().map(|segment| (*segment).to_owned()).collect(),
            args,
        }
    }
}

#[derive(
    serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq, etas_core::RetainedSize,
)]
pub enum TypeDeclKind {
    Primitive,
    Struct,
    Enum,
    Wrapper,
    Support,
    Spec,
    /// A non-nominal alias projected from a checked source declaration.
    Alias,
}
