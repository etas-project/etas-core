use super::StdType;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolDecl {
    pub name: String,
    pub params: Vec<StdType>,
    pub output: StdType,
    pub effects: Vec<String>,
    pub requirements: Vec<String>,
}
