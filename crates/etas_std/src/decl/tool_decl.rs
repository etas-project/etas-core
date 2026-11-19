use super::{StdEffectRef, StdType};

#[derive(
    serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq, etas_core::RetainedSize,
)]
pub struct ToolDecl {
    pub name: String,
    pub params: Vec<StdType>,
    pub output: StdType,
    pub effects: Vec<StdEffectRef>,
    pub requirements: Vec<String>,
}
