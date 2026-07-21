use super::std_type::StdType;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeDecl {
    pub name: String,
    pub params: Vec<TypeParam>,
    pub kind: TypeDeclKind,
    pub representation: Option<StdType>,
    pub derivable: bool,
}

impl TypeDecl {
    pub fn primitive(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            params: Vec::new(),
            kind: TypeDeclKind::Primitive,
            representation: None,
            derivable: false,
        }
    }

    pub fn generic(name: &str, params: &[&str], kind: TypeDeclKind) -> Self {
        Self {
            name: name.to_owned(),
            params: params.iter().map(|name| TypeParam::new(name)).collect(),
            kind,
            representation: None,
            derivable: false,
        }
    }

    pub fn with_representation(mut self, representation: StdType) -> Self {
        self.representation = Some(representation);
        self
    }

    pub fn derivable(mut self) -> Self {
        self.derivable = true;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeParam {
    pub name: String,
    pub bounds: Vec<String>,
}

impl TypeParam {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            bounds: Vec::new(),
        }
    }

    pub fn bounded(name: &str, bounds: &[&str]) -> Self {
        Self {
            name: name.to_owned(),
            bounds: bounds.iter().map(|bound| (*bound).to_owned()).collect(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeDeclKind {
    Primitive,
    Struct,
    Enum,
    Wrapper,
    Support,
    Spec,
}
