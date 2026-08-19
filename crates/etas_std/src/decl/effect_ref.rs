use super::StdType;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StdEffectRef {
    pub path: Vec<String>,
    pub args: Vec<StdStaticArg>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StdStaticArg {
    Type(StdType),
    Path(Vec<String>),
    String(String),
    Int(String),
    Wildcard,
}

impl StdEffectRef {
    pub fn new(path: &[&str]) -> Self {
        Self {
            path: path.iter().map(|segment| (*segment).to_owned()).collect(),
            args: Vec::new(),
        }
    }

    pub fn with_args(path: &[&str], args: Vec<StdStaticArg>) -> Self {
        Self {
            path: path.iter().map(|segment| (*segment).to_owned()).collect(),
            args,
        }
    }

    pub fn typed(path: &[&str], ty: StdType) -> Self {
        Self::with_args(path, vec![StdStaticArg::Type(ty)])
    }

    pub fn wildcard(path: &[&str], arity: usize) -> Self {
        Self::with_args(path, vec![StdStaticArg::Wildcard; arity])
    }
}

impl StdStaticArg {
    pub fn path(path: &[&str]) -> Self {
        Self::Path(path.iter().map(|segment| (*segment).to_owned()).collect())
    }
}
