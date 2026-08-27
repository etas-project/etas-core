#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StdType {
    Primitive(StdPrimitiveType),
    Var(String),
    Support(StdSupportConstraint),
    Array(Box<StdType>),
    List(Box<StdType>),
    Map {
        key: Box<StdType>,
        value: Box<StdType>,
    },
    Set(Box<StdType>),
    Range(Box<StdType>),
    Slice(Box<StdType>),
    Option(Box<StdType>),
    Result {
        ok: Box<StdType>,
        err: Box<StdType>,
    },
    Tuple(Vec<StdType>),
    Schema(Box<StdType>),
    Trust {
        wrapper: StdTrustWrapper,
        inner: Box<StdType>,
    },
    Prompt,
    PromptPart,
    Message(Box<StdType>),
    MemorySelection(Box<StdType>),
    Store {
        key: Box<StdType>,
        value: Box<StdType>,
    },
    MemoryRegion(Box<StdType>),
    ResourceHandleMemoryRegion(Box<StdType>),
    Named(String),
    NamedApplied {
        name: String,
        args: Vec<StdType>,
    },
    Record(Vec<StdRecordField>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StdRecordField {
    pub name: String,
    pub ty: StdType,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StdTrustWrapper {
    Trusted,
    Untrusted,
    Secret,
    Public,
    Sanitized,
}

impl StdRecordField {
    pub fn new(name: &str, ty: StdType) -> Self {
        Self {
            name: name.to_owned(),
            ty,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StdSupportConstraint {
    Index,
    LengthInput,
    EmptinessInput,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StdPrimitiveType {
    Bool,
    I8,
    I16,
    I32,
    I64,
    I128,
    ISize,
    U8,
    U16,
    U32,
    U64,
    U128,
    USize,
    F32,
    F64,
    Char,
    String,
    Bytes,
    Unit,
    Never,
}

impl StdType {
    pub fn parse(text: &str) -> Self {
        let text = text.trim();
        match text {
            "bool" => Self::Primitive(StdPrimitiveType::Bool),
            "i8" => Self::Primitive(StdPrimitiveType::I8),
            "i16" => Self::Primitive(StdPrimitiveType::I16),
            "i32" => Self::Primitive(StdPrimitiveType::I32),
            "i64" => Self::Primitive(StdPrimitiveType::I64),
            "i128" => Self::Primitive(StdPrimitiveType::I128),
            "isize" => Self::Primitive(StdPrimitiveType::ISize),
            "u8" => Self::Primitive(StdPrimitiveType::U8),
            "u16" => Self::Primitive(StdPrimitiveType::U16),
            "u32" => Self::Primitive(StdPrimitiveType::U32),
            "u64" => Self::Primitive(StdPrimitiveType::U64),
            "u128" => Self::Primitive(StdPrimitiveType::U128),
            "usize" => Self::Primitive(StdPrimitiveType::USize),
            "f32" => Self::Primitive(StdPrimitiveType::F32),
            "f64" => Self::Primitive(StdPrimitiveType::F64),
            "char" => Self::Primitive(StdPrimitiveType::Char),
            "string" => Self::Primitive(StdPrimitiveType::String),
            "bytes" => Self::Primitive(StdPrimitiveType::Bytes),
            "unit" => Self::Primitive(StdPrimitiveType::Unit),
            "never" => Self::Primitive(StdPrimitiveType::Never),
            "Index" => Self::Support(StdSupportConstraint::Index),
            "LengthInput" => Self::Support(StdSupportConstraint::LengthInput),
            "EmptinessInput" => Self::Support(StdSupportConstraint::EmptinessInput),
            "Prompt" => Self::Prompt,
            "PromptPart" => Self::PromptPart,
            _ => {
                if let Some(inner) = text
                    .strip_prefix('(')
                    .and_then(|rest| rest.strip_suffix(')'))
                    && inner.contains(',')
                {
                    return Self::Tuple(
                        split_type_args(inner)
                            .into_iter()
                            .map(Self::parse)
                            .collect(),
                    );
                }
                if is_type_variable(text) {
                    return Self::Var(text.to_owned());
                }
                if let Some(inner) = strip_type_constructor(text, "List") {
                    return Self::List(Box::new(Self::parse(inner)));
                }
                if let Some(inner) = strip_type_constructor(text, "Array") {
                    return Self::Array(Box::new(Self::parse(inner)));
                }
                if let Some(inner) = strip_type_constructor(text, "Map") {
                    let parts = split_type_args(inner);
                    assert!(parts.len() == 2, "invalid std Map type descriptor `{text}`");
                    return Self::Map {
                        key: Box::new(Self::parse(parts[0])),
                        value: Box::new(Self::parse(parts[1])),
                    };
                }
                if let Some(inner) = strip_type_constructor(text, "Set") {
                    return Self::Set(Box::new(Self::parse(inner)));
                }
                if let Some(inner) = strip_type_constructor(text, "Range") {
                    return Self::Range(Box::new(Self::parse(inner)));
                }
                if let Some(inner) = strip_type_constructor(text, "Slice") {
                    return Self::Slice(Box::new(Self::parse(inner)));
                }
                if let Some(inner) = strip_type_constructor(text, "Option") {
                    return Self::Option(Box::new(Self::parse(inner)));
                }
                if let Some(inner) = strip_type_constructor(text, "Schema") {
                    return Self::Schema(Box::new(Self::parse(inner)));
                }
                for (constructor, wrapper) in [
                    ("Trusted", StdTrustWrapper::Trusted),
                    ("Untrusted", StdTrustWrapper::Untrusted),
                    ("Secret", StdTrustWrapper::Secret),
                    ("Public", StdTrustWrapper::Public),
                    ("Sanitized", StdTrustWrapper::Sanitized),
                ] {
                    if let Some(inner) = strip_type_constructor(text, constructor) {
                        return Self::Trust {
                            wrapper,
                            inner: Box::new(Self::parse(inner)),
                        };
                    }
                }
                if let Some(inner) = strip_type_constructor(text, "Message") {
                    return Self::Message(Box::new(Self::parse(inner)));
                }
                if let Some(inner) = strip_type_constructor(text, "MemorySelection") {
                    return Self::MemorySelection(Box::new(Self::parse(inner)));
                }
                if let Some(inner) = strip_type_constructor(text, "MemoryRegion") {
                    return Self::MemoryRegion(Box::new(Self::parse(inner)));
                }
                if let Some(inner) = strip_type_constructor(text, "ResourceHandleMemoryRegion") {
                    return Self::ResourceHandleMemoryRegion(Box::new(Self::parse(inner)));
                }
                if let Some(inner) = strip_type_constructor(text, "Store") {
                    let parts = split_type_args(inner);
                    assert!(
                        parts.len() == 2,
                        "invalid std Store type descriptor `{text}`"
                    );
                    return Self::Store {
                        key: Box::new(Self::parse(parts[0])),
                        value: Box::new(Self::parse(parts[1])),
                    };
                }
                if let Some(inner) = strip_type_constructor(text, "Result") {
                    let parts = split_type_args(inner);
                    assert!(
                        parts.len() == 2,
                        "invalid std Result type descriptor `{text}`"
                    );
                    return Self::Result {
                        ok: Box::new(Self::parse(parts[0])),
                        err: Box::new(Self::parse(parts[1])),
                    };
                }
                if let Some((name, inner)) = strip_any_type_constructor(text) {
                    return Self::NamedApplied {
                        name: name.to_owned(),
                        args: split_type_args(inner)
                            .into_iter()
                            .map(Self::parse)
                            .collect(),
                    };
                }
                Self::Named(text.to_owned())
            }
        }
    }
}

impl StdSupportConstraint {
    pub const fn source_name(self) -> &'static str {
        match self {
            Self::Index => "Index",
            Self::LengthInput => "LengthInput",
            Self::EmptinessInput => "EmptinessInput",
        }
    }
}

fn is_type_variable(text: &str) -> bool {
    let mut chars = text.chars();
    matches!(chars.next(), Some(ch) if ch.is_ascii_uppercase()) && chars.next().is_none()
}

fn strip_type_constructor<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    text.strip_prefix(name)
        .and_then(|rest| rest.strip_prefix('['))
        .and_then(|rest| rest.strip_suffix(']'))
}

fn strip_any_type_constructor(text: &str) -> Option<(&str, &str)> {
    let open = text.find('[')?;
    let close = text.strip_suffix(']')?;
    let name = &text[..open];
    if name.is_empty()
        || !name.split('.').all(|segment| {
            !segment.is_empty()
                && segment
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        })
    {
        return None;
    }
    Some((name, &close[open + 1..]))
}

fn split_type_args(text: &str) -> Vec<&str> {
    let mut bracket_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut start = 0usize;
    let mut parts = Vec::new();
    for (index, ch) in text.char_indices() {
        match ch {
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            ',' if bracket_depth == 0 && paren_depth == 0 => {
                parts.push(text[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push(text[start..].trim());
    parts
}
