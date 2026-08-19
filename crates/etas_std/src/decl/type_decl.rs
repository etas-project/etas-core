use super::std_type::StdType;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeDecl {
    pub name: String,
    pub params: Vec<StdGenericParam>,
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
            params: params
                .iter()
                .map(|name| StdGenericParam::new(name))
                .collect(),
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

#[derive(Clone, Debug, PartialEq, Eq)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeDeclKind {
    Primitive,
    Struct,
    Enum,
    Wrapper,
    Support,
    Spec,
}
