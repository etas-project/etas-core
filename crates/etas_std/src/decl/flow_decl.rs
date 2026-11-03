use super::{StdEffectRef, StdGenericParam, StdType};

#[derive(Clone, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub struct FlowDecl {
    pub name: String,
    pub type_params: Vec<StdGenericParam>,
    pub params: Vec<StdType>,
    pub output: StdType,
    pub public_effects: Vec<StdEffectRef>,
    pub requested_actions: Vec<StdEffectRef>,
    pub source_method: Option<FlowSourceMethod>,
}

#[derive(Clone, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub struct FlowSourceMethod {
    pub receiver: StdType,
    pub name: String,
    pub kind: FlowSourceMethodKind,
    pub operation: FlowSourceMethodOperation,
    pub owner_type_params: Vec<String>,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    etas_core::RetainedSize,
)]
pub enum FlowSourceMethodOperation {
    #[default]
    Call,
    CheckedIndex,
    CheckedIntrinsic,
    RejectSecretData,
    CheckedMessageCast,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub enum FlowSourceMethodKind {
    TypeMember,
    ValueMethod,
}

impl FlowDecl {
    pub fn pure(name: &str, params: &[&str], output: &str) -> Self {
        Self {
            name: name.to_owned(),
            type_params: Vec::new(),
            params: params.iter().map(|param| StdType::parse(param)).collect(),
            output: StdType::parse(output),
            public_effects: Vec::new(),
            requested_actions: Vec::new(),
            source_method: None,
        }
    }

    pub fn effectful(
        name: &str,
        params: &[&str],
        output: &str,
        public_effects: &[StdEffectRef],
    ) -> Self {
        Self::with_actions(name, params, output, public_effects, &[])
    }

    pub fn with_actions(
        name: &str,
        params: &[&str],
        output: &str,
        public_effects: &[StdEffectRef],
        requested_actions: &[StdEffectRef],
    ) -> Self {
        Self::with_type_params_actions(name, &[], params, output, public_effects, requested_actions)
    }

    pub fn with_type_params_actions(
        name: &str,
        type_params: &[StdGenericParam],
        params: &[&str],
        output: &str,
        public_effects: &[StdEffectRef],
        requested_actions: &[StdEffectRef],
    ) -> Self {
        Self {
            name: name.to_owned(),
            type_params: type_params.to_vec(),
            params: params.iter().map(|param| StdType::parse(param)).collect(),
            output: StdType::parse(output),
            public_effects: public_effects.to_vec(),
            requested_actions: requested_actions.to_vec(),
            source_method: None,
        }
    }

    pub fn with_type_member_method(mut self, receiver: &str, name: &str) -> Self {
        self.source_method = Some(FlowSourceMethod {
            receiver: StdType::parse(receiver),
            name: name.to_owned(),
            kind: FlowSourceMethodKind::TypeMember,
            operation: FlowSourceMethodOperation::Call,
            owner_type_params: Vec::new(),
        });
        self
    }

    pub fn with_value_method(mut self, receiver: &str, name: &str) -> Self {
        self.source_method = Some(FlowSourceMethod {
            receiver: StdType::parse(receiver),
            name: name.to_owned(),
            kind: FlowSourceMethodKind::ValueMethod,
            operation: FlowSourceMethodOperation::Call,
            owner_type_params: Vec::new(),
        });
        self
    }

    pub fn with_checked_index_method(mut self, receiver: &str, name: &str) -> Self {
        self.source_method = Some(FlowSourceMethod {
            receiver: StdType::parse(receiver),
            name: name.to_owned(),
            kind: FlowSourceMethodKind::ValueMethod,
            operation: FlowSourceMethodOperation::CheckedIndex,
            owner_type_params: Vec::new(),
        });
        self
    }

    pub fn with_intrinsic_value_method(mut self, receiver: StdType, name: &str) -> Self {
        self.source_method = Some(FlowSourceMethod {
            receiver,
            name: name.to_owned(),
            kind: FlowSourceMethodKind::ValueMethod,
            operation: FlowSourceMethodOperation::CheckedIntrinsic,
            owner_type_params: Vec::new(),
        });
        self
    }

    pub fn with_method_owner_parameters(mut self, names: &[&str]) -> Self {
        self.source_method
            .as_mut()
            .expect("method descriptor must be declared first")
            .owner_type_params = names.iter().map(|name| (*name).into()).collect();
        self
    }

    pub fn with_method_operation(mut self, operation: FlowSourceMethodOperation) -> Self {
        self.source_method
            .as_mut()
            .expect("method descriptor must be declared first")
            .operation = operation;
        self
    }
}
