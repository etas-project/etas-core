use super::StdType;

#[derive(Clone, Debug, PartialEq, Eq)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequirementKind {
    Policy,
    Schema,
    Evidence,
    Limit,
    Predicate,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RequirementSemantics {
    #[default]
    None,
    Limit(StdLimitKind),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StdLimitKind {
    Iterations,
    Tokens,
    ContextTokens,
    Cost,
    WallTime,
    Attempts,
}
