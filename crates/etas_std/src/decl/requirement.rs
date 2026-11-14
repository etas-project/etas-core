use super::StdType;

#[derive(serde::Serialize, Clone, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub struct RequirementDecl {
    pub name: String,
    pub params: Vec<StdType>,
    pub kind: RequirementKind,
    pub semantics: RequirementSemantics,
}

impl RequirementDecl {
    pub fn new(name: &str, params: &[&str], kind: RequirementKind) -> Self {
        Self {
            name: name.to_owned(),
            params: params.iter().map(|param| StdType::parse(param)).collect(),
            kind,
            semantics: RequirementSemantics::None,
        }
    }

    pub fn limit(name: &str, params: &[&str], kind: StdLimitKind) -> Self {
        Self {
            name: name.to_owned(),
            params: params.iter().map(|param| StdType::parse(param)).collect(),
            kind: RequirementKind::Limit,
            semantics: RequirementSemantics::Limit(kind),
        }
    }
}

#[derive(serde::Serialize, Clone, Copy, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub enum RequirementKind {
    Policy,
    Schema,
    Evidence,
    Limit,
    Predicate,
}

#[derive(serde::Serialize, Clone, Copy, Debug, Default, PartialEq, Eq, etas_core::RetainedSize)]
pub enum RequirementSemantics {
    #[default]
    None,
    Limit(StdLimitKind),
}

#[derive(serde::Serialize, Clone, Copy, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub enum StdLimitKind {
    Iterations,
    Tokens,
    ContextTokens,
    Cost,
    WallTime,
    Attempts,
    Concurrency,
}
