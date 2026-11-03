use super::std_type::StdType;

#[derive(Clone, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub struct TypeDecl {
    pub name: String,
    pub params: Vec<StdGenericParam>,
    pub kind: TypeDeclKind,
    pub representation: Option<StdType>,
    /// Canonical compiler type for a support declaration, not nominal storage.
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
        Self {
            name: name.to_owned(),
            params: params
                .iter()
                .map(|name| StdGenericParam::new(name))
                .collect(),
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, etas_core::RetainedSize)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub enum StdTypeConstruction {
    Public,
    IntrinsicOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, etas_core::RetainedSize)]
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

#[derive(Clone, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub struct StdGenericParam {
    pub name: String,
    pub bounds: Vec<StdSpecRef>,
}

impl StdGenericParam {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            bounds: Vec::new(),
        }
    }

    pub fn bounded(name: &str, bounds: &[StdSpecRef]) -> Self {
        Self {
            name: name.to_owned(),
            bounds: bounds.to_vec(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, etas_core::RetainedSize)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub enum TypeDeclKind {
    Primitive,
    Struct,
    Enum,
    Wrapper,
    Support,
    Spec,
}
