use super::StdType;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValueDecl {
    pub name: String,
    pub ty: StdType,
}

impl ValueDecl {
    pub fn new(name: &str, ty: &str) -> Self {
        Self {
            name: name.to_owned(),
            ty: StdType::parse(ty),
        }
    }
}
