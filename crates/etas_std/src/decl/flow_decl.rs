use super::{StdEffectRef, StdGenericParam, StdType};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlowDecl {
    pub name: String,
    pub type_params: Vec<StdGenericParam>,
    pub params: Vec<StdType>,
    pub output: StdType,
    pub public_effects: Vec<StdEffectRef>,
    pub requested_actions: Vec<StdEffectRef>,
    pub source_method: Option<FlowSourceMethod>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlowSourceMethod {
    pub receiver: StdType,
    pub name: String,
    pub kind: FlowSourceMethodKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
        });
        self
    }

    pub fn with_value_method(mut self, receiver: &str, name: &str) -> Self {
        self.source_method = Some(FlowSourceMethod {
            receiver: StdType::parse(receiver),
            name: name.to_owned(),
            kind: FlowSourceMethodKind::ValueMethod,
        });
        self
    }
}
