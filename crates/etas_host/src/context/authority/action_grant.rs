use crate::HostValue;

#[derive(Clone, Debug, PartialEq)]
pub enum HostActionGrant {
    Allow(ActionPattern),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ActionPattern {
    Exact(ActionInstance),
    Pattern {
        effect: String,
        action: String,
        args: Vec<ActionArgPattern>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActionInstance {
    pub effect: String,
    pub action: String,
    pub args: Vec<HostValue>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ActionArgPattern {
    Any,
    Exact(HostValue),
    Prefix(Vec<String>),
}

impl HostActionGrant {
    pub fn allow(effect: impl Into<String>, action: impl Into<String>) -> Self {
        Self::Allow(ActionPattern::Pattern {
            effect: effect.into(),
            action: action.into(),
            args: Vec::new(),
        })
    }

    pub fn allow_with_args(
        effect: impl Into<String>,
        action: impl Into<String>,
        args: Vec<ActionArgPattern>,
    ) -> Self {
        Self::Allow(ActionPattern::Pattern {
            effect: effect.into(),
            action: action.into(),
            args,
        })
    }

    pub fn allows(&self, action: &ActionInstance) -> bool {
        match self {
            Self::Allow(pattern) => pattern.matches(action),
        }
    }
}

impl ActionPattern {
    pub fn matches(&self, action: &ActionInstance) -> bool {
        match self {
            Self::Exact(expected) => expected == action,
            Self::Pattern {
                effect,
                action: pattern_action,
                args,
            } => {
                effect == &action.effect
                    && pattern_action == &action.action
                    && args_match(args, &action.args)
            }
        }
    }
}

impl ActionInstance {
    pub fn new(effect: impl Into<String>, action: impl Into<String>, args: Vec<HostValue>) -> Self {
        Self {
            effect: effect.into(),
            action: action.into(),
            args,
        }
    }
}

fn args_match(patterns: &[ActionArgPattern], values: &[HostValue]) -> bool {
    if patterns.len() > values.len() {
        return false;
    }
    patterns
        .iter()
        .zip(values.iter())
        .all(|(pattern, value)| pattern.matches(value))
}

impl ActionArgPattern {
    fn matches(&self, value: &HostValue) -> bool {
        match self {
            Self::Any => true,
            Self::Exact(expected) => expected == value,
            Self::Prefix(prefix) => match value {
                HostValue::String(text) => text.starts_with(&prefix.join(".")),
                HostValue::List(parts) => {
                    if prefix.len() > parts.len() {
                        return false;
                    }
                    prefix.iter().zip(parts.iter()).all(|(expected, actual)| {
                        matches!(actual, HostValue::String(actual) if actual == expected)
                    })
                }
                _ => false,
            },
        }
    }
}
