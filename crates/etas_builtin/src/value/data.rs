use super::BuiltinTypeTag;

#[derive(Clone, Debug, PartialEq)]
pub enum BuiltinValue {
    Unit,
    Bool(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    Isize(isize),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    Usize(usize),
    F32(f32),
    F64(f64),
    Char(char),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<BuiltinValue>),
    List(Vec<BuiltinValue>),
    Map(Vec<(BuiltinValue, BuiltinValue)>),
    Record(Vec<(String, BuiltinValue)>),
    Set(Vec<BuiltinValue>),
    Range {
        start: Box<BuiltinValue>,
        end: Box<BuiltinValue>,
        bounds: BuiltinRangeBounds,
    },
    Slice(Vec<BuiltinValue>),
    OptionSome(Box<BuiltinValue>),
    OptionNone,
    ResultOk(Box<BuiltinValue>),
    ResultErr(Box<BuiltinValue>),
    Variant {
        name: String,
        fields: Vec<BuiltinValue>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuiltinRangeBounds {
    ClosedClosed,
    ClosedOpen,
    OpenOpen,
    OpenClosed,
}

impl BuiltinValue {
    pub fn type_tag(&self) -> BuiltinTypeTag {
        match self {
            Self::Unit => BuiltinTypeTag::Unit,
            Self::Bool(_) => BuiltinTypeTag::Bool,
            Self::I8(_) => BuiltinTypeTag::I8,
            Self::I16(_) => BuiltinTypeTag::I16,
            Self::I32(_) => BuiltinTypeTag::I32,
            Self::I64(_) => BuiltinTypeTag::I64,
            Self::I128(_) => BuiltinTypeTag::I128,
            Self::Isize(_) => BuiltinTypeTag::Isize,
            Self::U8(_) => BuiltinTypeTag::U8,
            Self::U16(_) => BuiltinTypeTag::U16,
            Self::U32(_) => BuiltinTypeTag::U32,
            Self::U64(_) => BuiltinTypeTag::U64,
            Self::U128(_) => BuiltinTypeTag::U128,
            Self::Usize(_) => BuiltinTypeTag::Usize,
            Self::F32(_) => BuiltinTypeTag::F32,
            Self::F64(_) => BuiltinTypeTag::F64,
            Self::Char(_) => BuiltinTypeTag::Char,
            Self::String(_) => BuiltinTypeTag::String,
            Self::Bytes(_) => BuiltinTypeTag::Bytes,
            Self::Array(_) => BuiltinTypeTag::Array,
            Self::List(_) => BuiltinTypeTag::List,
            Self::Map(_) => BuiltinTypeTag::Map,
            Self::Record(_) => BuiltinTypeTag::Record,
            Self::Set(_) => BuiltinTypeTag::Set,
            Self::Range { .. } => BuiltinTypeTag::Range,
            Self::Slice(_) => BuiltinTypeTag::Slice,
            Self::OptionSome(_) | Self::OptionNone => BuiltinTypeTag::Option,
            Self::ResultOk(_) | Self::ResultErr(_) => BuiltinTypeTag::Result,
            Self::Variant { .. } => BuiltinTypeTag::Variant,
        }
    }
}
