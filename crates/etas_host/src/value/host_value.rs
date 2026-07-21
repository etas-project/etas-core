#[derive(Clone, Debug, PartialEq)]
pub enum HostValue {
    Unit,
    Bool(bool),
    Int(i128),
    UInt(u128),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    List(Vec<HostValue>),
    Map(Vec<(HostValue, HostValue)>),
    Record(Vec<(String, HostValue)>),
    Variant {
        name: String,
        fields: Vec<HostValue>,
    },
    Json(HostJsonValue),
}

#[derive(Clone, Debug, PartialEq)]
pub enum HostJsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<HostJsonValue>),
    Object(Vec<(String, HostJsonValue)>),
}
