use super::StdType;

#[derive(serde::Serialize, Clone, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub struct StdEffectRef {
    pub path: Vec<String>,
    pub args: Vec<StdStaticArg>,
}

/// A standard-library effect row. `tail` names an `effect` generic binder.
#[derive(serde::Serialize, Clone, Debug, Default, PartialEq, Eq, etas_core::RetainedSize)]
pub struct StdEffectRowExpr {
    pub effects: Vec<StdEffectRef>,
    pub tail: Option<String>,
}

impl StdEffectRowExpr {
    pub fn closed(effects: Vec<StdEffectRef>) -> Self {
        Self {
            effects,
            tail: None,
        }
    }

    pub fn param(name: &str) -> Self {
        Self {
            effects: Vec::new(),
            tail: Some(name.to_owned()),
        }
    }

    pub fn with_tail(effects: Vec<StdEffectRef>, tail: &str) -> Self {
        Self {
            effects,
            tail: Some(tail.to_owned()),
        }
    }
}

#[derive(serde::Serialize, Clone, Debug, PartialEq, Eq, etas_core::RetainedSize)]
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
