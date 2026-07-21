#[derive(Clone, Debug, PartialEq)]
pub enum HostSchema {
    Unit,
    Bool,
    Int,
    UInt,
    Float,
    String,
    Bytes,
    List(Box<HostSchema>),
    Map {
        key: Box<HostSchema>,
        value: Box<HostSchema>,
    },
    Record(Vec<HostFieldSchema>),
    Variant(Vec<HostVariantSchema>),
    Json,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HostFieldSchema {
    pub name: String,
    pub schema: HostSchema,
    pub optional: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HostVariantSchema {
    pub name: String,
    pub fields: Vec<HostSchema>,
}
