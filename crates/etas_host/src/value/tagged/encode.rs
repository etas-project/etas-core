use super::{limit, schema};
use crate::{HostError, HostJsonValue, HostValue, StorageLimits};
use std::io::Write;
pub(crate) fn encode(value: &HostValue, limits: &StorageLimits) -> Result<String, HostError> {
    let mut bytes = Vec::new();
    encode_to(value, limits, &mut bytes)?;
    String::from_utf8(bytes).map_err(|_| schema("stored value encoding is not UTF-8"))
}
pub(crate) fn encode_to(
    value: &HostValue,
    limits: &StorageLimits,
    writer: impl Write,
) -> Result<usize, HostError> {
    limits.value_size(value)?;
    let mut out = Output::new(writer, limits.max_value_bytes);
    out.host(value)?;
    Ok(out.used)
}
pub(super) struct Output<W> {
    writer: W,
    pub(super) used: usize,
    max: usize,
}
impl<W: Write> Write for Output<W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self
            .used
            .checked_add(bytes.len())
            .is_none_or(|size| size > self.max)
        {
            return Err(std::io::Error::other(
                "stored value exceeds codec byte limit",
            ));
        }
        self.writer.write_all(bytes)?;
        self.used += bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl<W: Write> Output<W> {
    pub(super) fn new(writer: W, max: usize) -> Self {
        Self {
            writer,
            used: 0,
            max,
        }
    }
    pub(super) fn raw(&mut self, value: &str) -> Result<(), HostError> {
        self.write_all(value.as_bytes()).map_err(|_| limit())
    }
    pub(super) fn json<T: serde::Serialize + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), HostError> {
        serde_json::to_writer(self, value).map_err(|_| limit())
    }
    fn sequence<T>(
        &mut self,
        values: &[T],
        mut item: impl FnMut(&mut Self, &T) -> Result<(), HostError>,
    ) -> Result<(), HostError> {
        self.raw("[")?;
        for (index, value) in values.iter().enumerate() {
            if index > 0 {
                self.raw(",")?;
            }
            item(self, value)?;
        }
        self.raw("]")
    }
    pub(super) fn scalar<T: serde::Serialize + ?Sized>(
        &mut self,
        kind: &str,
        value: &T,
    ) -> Result<(), HostError> {
        self.raw("{\"kind\":")?;
        self.json(kind)?;
        self.raw(",\"value\":")?;
        self.json(value)?;
        self.raw("}")
    }
    pub(super) fn host(&mut self, value: &HostValue) -> Result<(), HostError> {
        match value {
            HostValue::Unit => self.raw("{\"kind\":\"unit\"}"),
            HostValue::Bool(value) => self.scalar("bool", value),
            HostValue::Int(value) => self.scalar("int", &value.to_string()),
            HostValue::UInt(value) => self.scalar("uint", &value.to_string()),
            HostValue::Float(value) if value.is_finite() => self.scalar("float", value),
            HostValue::Float(_) => Err(schema("non-finite host float cannot be stored")),
            HostValue::String(value) => self.scalar("string", value),
            HostValue::Bytes(value) => self.scalar("bytes", value),
            HostValue::List(values) => {
                self.raw("{\"items\":")?;
                self.sequence(values, Self::host)?;
                self.raw(",\"kind\":\"list\"}")
            }
            HostValue::Map(values) => {
                self.raw("{\"entries\":")?;
                self.sequence(values, |out, (key, value)| {
                    out.raw("{\"key\":")?;
                    out.host(key)?;
                    out.raw(",\"value\":")?;
                    out.host(value)?;
                    out.raw("}")
                })?;
                self.raw(",\"kind\":\"map\"}")
            }
            HostValue::Record(values) => {
                let mut seen = std::collections::BTreeSet::new();
                for (name, _) in values {
                    if !seen.insert(name) {
                        return Err(schema("duplicate stored record field"));
                    }
                }
                self.raw("{\"fields\":")?;
                self.sequence(values, |out, (name, value)| {
                    out.raw("{\"name\":")?;
                    out.json(name)?;
                    out.raw(",\"value\":")?;
                    out.host(value)?;
                    out.raw("}")
                })?;
                self.raw(",\"kind\":\"record\"}")
            }
            HostValue::Variant { name, fields } => {
                self.raw("{\"fields\":")?;
                self.sequence(fields, Self::host)?;
                self.raw(",\"kind\":\"variant\",\"name\":")?;
                self.json(name)?;
                self.raw("}")
            }
            HostValue::Json(value) => {
                self.raw("{\"kind\":\"json\",\"value\":")?;
                self.json_value(value)?;
                self.raw("}")
            }
        }
    }
    fn json_value(&mut self, value: &HostJsonValue) -> Result<(), HostError> {
        match value {
            HostJsonValue::Null => self.raw("null"),
            HostJsonValue::Bool(value) => self.json(value),
            HostJsonValue::Number(value) if value.is_finite() => self.json(value),
            HostJsonValue::Number(_) => Err(schema("non-finite stored JSON number")),
            HostJsonValue::String(value) => self.json(value),
            HostJsonValue::Array(values) => self.sequence(values, Self::json_value),
            HostJsonValue::Object(values) => {
                let mut ordered = std::collections::BTreeMap::new();
                for (name, value) in values {
                    if ordered.insert(name, value).is_some() {
                        return Err(schema("duplicate stored JSON field"));
                    }
                }
                self.raw("{")?;
                for (index, (name, value)) in ordered.into_iter().enumerate() {
                    if index > 0 {
                        self.raw(",")?;
                    }
                    self.json(name)?;
                    self.raw(":")?;
                    self.json_value(value)?;
                }
                self.raw("}")
            }
        }
    }
}
