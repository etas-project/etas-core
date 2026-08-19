use super::{StdEffectRef, StdType};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolDecl {
    pub name: String,
    pub params: Vec<StdType>,
    pub output: StdType,
    pub effects: Vec<StdEffectRef>,
    pub requirements: Vec<String>,
}
