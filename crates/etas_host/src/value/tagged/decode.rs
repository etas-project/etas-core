use super::{limit, schema};
use crate::{HostError, HostJsonValue, HostValue, StorageLimits};
use serde::{
    Deserialize,
    de::{DeserializeSeed, Error, MapAccess, SeqAccess, Visitor},
};
use serde_json::value::RawValue;

pub(crate) fn decode(text: &str, limits: &StorageLimits) -> Result<HostValue, HostError> {
    if text.len() > limits.max_value_bytes {
        return Err(limit());
    }
    // Bound wire nesting before RawValue scans any subtree.
    let mut depth = 0usize;
    let mut quoted = false;
    let mut escaped = false;
    for byte in text.bytes() {
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
            continue;
        }
        match byte {
            b'"' => quoted = true,
            b'[' | b'{' => {
                depth += 1;
                if depth > limits.max_depth * 2 + 4 {
                    return Err(limit());
                }
            }
            b']' | b'}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    let raw: &RawValue = serde_json::from_str(text).map_err(|_| schema("invalid stored JSON"))?;
    let mut budget = Budget { limits, nodes: 0 };
    let value = budget.host(raw, 0)?;
    limits.value_size(&value)?;
    Ok(value)
}
struct Budget<'a> {
    limits: &'a StorageLimits,
    nodes: usize,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Tagged<'a> {
    kind: String,
    #[serde(default, borrow, deserialize_with = "present")]
    value: Option<&'a RawValue>,
    #[serde(default, borrow, deserialize_with = "present")]
    items: Option<&'a RawValue>,
    #[serde(default, borrow, deserialize_with = "present")]
    entries: Option<&'a RawValue>,
    #[serde(default, borrow, deserialize_with = "present")]
    fields: Option<&'a RawValue>,
    name: Option<String>,
}
fn present<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<&'de RawValue>, D::Error> {
    <&RawValue>::deserialize(d).map(Some)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Pair<'a> {
    #[serde(borrow)]
    key: &'a RawValue,
    #[serde(borrow)]
    value: &'a RawValue,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Field<'a> {
    name: String,
    #[serde(borrow)]
    value: &'a RawValue,
}
fn parsed<'a, T: Deserialize<'a>>(raw: &'a RawValue) -> Result<T, HostError> {
    serde_json::from_str(raw.get()).map_err(|_| schema("stored value has invalid field shape"))
}
fn required(raw: Option<&RawValue>) -> Result<&RawValue, HostError> {
    raw.ok_or_else(|| schema("stored value is missing required field"))
}
impl Budget<'_> {
    fn visit(&mut self, depth: usize) -> Result<(), HostError> {
        self.nodes = self.nodes.checked_add(1).ok_or_else(limit)?;
        if self.nodes > self.limits.max_nodes || depth > self.limits.max_depth {
            return Err(limit());
        }
        Ok(())
    }
    fn host(&mut self, raw: &RawValue, depth: usize) -> Result<HostValue, HostError> {
        self.visit(depth)?;
        let item: Tagged<'_> = parsed(raw)?;
        let mask = u8::from(item.value.is_some())
            + 2 * u8::from(item.items.is_some())
            + 4 * u8::from(item.entries.is_some())
            + 8 * u8::from(item.fields.is_some())
            + 16 * u8::from(item.name.is_some());
        let expected = match item.kind.as_str() {
            "unit" => 0,
            "list" => 2,
            "map" => 4,
            "record" => 8,
            "variant" => 24,
            "bool" | "int" | "uint" | "float" | "string" | "bytes" | "json" => 1,
            _ => return Err(schema("unknown stored value kind")),
        };
        if mask != expected {
            return Err(schema("stored value fields do not match kind"));
        }
        match item.kind.as_str() {
            "unit" => Ok(HostValue::Unit),
            "bool" => parsed(required(item.value)?).map(HostValue::Bool),
            "int" => parsed::<String>(required(item.value)?)?
                .parse()
                .map(HostValue::Int)
                .map_err(|_| schema("stored integer out of range")),
            "uint" => parsed::<String>(required(item.value)?)?
                .parse()
                .map(HostValue::UInt)
                .map_err(|_| schema("stored integer out of range")),
            "float" => {
                let value: f64 = parsed(required(item.value)?)?;
                if !value.is_finite() {
                    return Err(schema("non-finite stored float"));
                }
                Ok(HostValue::Float(value))
            }
            "string" => parsed(required(item.value)?).map(HostValue::String),
            "bytes" => {
                sequence(required(item.value)?, |raw| parsed::<u8>(raw)).map(HostValue::Bytes)
            }
            "list" => sequence(required(item.items)?, |raw| self.host(raw, depth + 1))
                .map(HostValue::List),
            "map" => sequence(required(item.entries)?, |raw| {
                let pair: Pair<'_> = parsed(raw)?;
                Ok((
                    self.host(pair.key, depth + 1)?,
                    self.host(pair.value, depth + 1)?,
                ))
            })
            .map(HostValue::Map),
            "record" => {
                let mut seen = std::collections::BTreeSet::new();
                sequence(required(item.fields)?, |raw| {
                    let field: Field<'_> = parsed(raw)?;
                    if !seen.insert(field.name.clone()) {
                        return Err(schema("duplicate stored record field"));
                    }
                    Ok((field.name, self.host(field.value, depth + 1)?))
                })
                .map(HostValue::Record)
            }
            "variant" => Ok(HostValue::Variant {
                name: item
                    .name
                    .ok_or_else(|| schema("stored variant missing name"))?,
                fields: sequence(required(item.fields)?, |raw| self.host(raw, depth + 1))?,
            }),
            "json" => self
                .json(required(item.value)?, depth + 1)
                .map(HostValue::Json),
            _ => Err(schema("unknown stored value kind")),
        }
    }
    fn json(&mut self, raw: &RawValue, depth: usize) -> Result<HostJsonValue, HostError> {
        self.visit(depth)?;
        match raw.get().as_bytes().first() {
            Some(b'n') => {
                parsed::<()>(raw)?;
                Ok(HostJsonValue::Null)
            }
            Some(b't' | b'f') => parsed(raw).map(HostJsonValue::Bool),
            Some(b'"') => parsed(raw).map(HostJsonValue::String),
            Some(b'[') => sequence(raw, |raw| self.json(raw, depth + 1)).map(HostJsonValue::Array),
            Some(b'{') => object(raw, |raw| self.json(raw, depth + 1)).map(HostJsonValue::Object),
            _ => {
                let value: f64 = parsed(raw)?;
                if !value.is_finite() {
                    return Err(schema("non-finite stored JSON number"));
                }
                Ok(HostJsonValue::Number(value))
            }
        }
    }
}
fn sequence<T>(
    raw: &RawValue,
    mut decode: impl FnMut(&RawValue) -> Result<T, HostError>,
) -> Result<Vec<T>, HostError> {
    struct Elements<'a, F, T> {
        decode: &'a mut F,
        failure: &'a mut Option<HostError>,
        marker: std::marker::PhantomData<T>,
    }
    impl<'de, F, T> Visitor<'de> for Elements<'_, F, T>
    where
        F: FnMut(&RawValue) -> Result<T, HostError>,
    {
        type Value = Vec<T>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("stored sequence")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut values = Vec::new();
            while let Some(raw) = seq.next_element::<&RawValue>()? {
                match (self.decode)(raw) {
                    Ok(value) => values.push(value),
                    Err(error) => {
                        *self.failure = Some(error);
                        return Err(A::Error::custom("invalid stored sequence item"));
                    }
                }
            }
            Ok(values)
        }
    }
    impl<'de, F, T> DeserializeSeed<'de> for Elements<'_, F, T>
    where
        F: FnMut(&RawValue) -> Result<T, HostError>,
    {
        type Value = Vec<T>;
        fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
            d.deserialize_seq(self)
        }
    }
    let mut failure = None;
    let mut deserializer = serde_json::Deserializer::from_str(raw.get());
    let result = Elements {
        decode: &mut decode,
        failure: &mut failure,
        marker: std::marker::PhantomData,
    }
    .deserialize(&mut deserializer);
    match result {
        Ok(value) => Ok(value),
        Err(_) => Err(failure.unwrap_or_else(|| schema("invalid stored sequence"))),
    }
}
fn object<T>(
    raw: &RawValue,
    mut decode: impl FnMut(&RawValue) -> Result<T, HostError>,
) -> Result<Vec<(String, T)>, HostError> {
    struct Fields<'a, F, T> {
        decode: &'a mut F,
        failure: &'a mut Option<HostError>,
        marker: std::marker::PhantomData<T>,
    }
    impl<'de, F, T> Visitor<'de> for Fields<'_, F, T>
    where
        F: FnMut(&RawValue) -> Result<T, HostError>,
    {
        type Value = Vec<(String, T)>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("stored object")
        }
        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut values = std::collections::BTreeMap::new();
            while let Some((key, raw)) = map.next_entry::<String, &RawValue>()? {
                if values.contains_key(&key) {
                    return Err(A::Error::custom("duplicate stored JSON field"));
                }
                match (self.decode)(raw) {
                    Ok(value) => {
                        values.insert(key, value);
                    }
                    Err(error) => {
                        *self.failure = Some(error);
                        return Err(A::Error::custom("invalid stored object item"));
                    }
                }
            }
            Ok(values.into_iter().collect())
        }
    }
    impl<'de, F, T> DeserializeSeed<'de> for Fields<'_, F, T>
    where
        F: FnMut(&RawValue) -> Result<T, HostError>,
    {
        type Value = Vec<(String, T)>;
        fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
            d.deserialize_map(self)
        }
    }
    let mut failure = None;
    let mut deserializer = serde_json::Deserializer::from_str(raw.get());
    let result = Fields {
        decode: &mut decode,
        failure: &mut failure,
        marker: std::marker::PhantomData,
    }
    .deserialize(&mut deserializer);
    match result {
        Ok(value) => Ok(value),
        Err(_) => Err(failure.unwrap_or_else(|| schema("invalid stored object"))),
    }
}
