use super::{encode::Output, schema};
use crate::{HostError, HostValue, StorageLimits};
use std::io::Write;

#[derive(Clone, Copy)]
pub(crate) enum RecordValueRef<'a> {
    String(&'a str),
    OptionalString(Option<&'a str>),
    Value(&'a HostValue),
    OptionalValue(Option<&'a HostValue>),
}
impl RecordValueRef<'_> {
    pub(crate) fn into_owned(self) -> HostValue {
        match self {
            Self::String(s) => HostValue::String(s.to_owned()),
            Self::Value(v) => v.clone(),
            Self::OptionalString(s) => owned_option(s.map(|s| HostValue::String(s.to_owned()))),
            Self::OptionalValue(v) => owned_option(v.cloned()),
        }
    }
}
fn owned_option(value: Option<HostValue>) -> HostValue {
    HostValue::Variant {
        name: if value.is_some() { "Some" } else { "None" }.to_owned(),
        fields: value.into_iter().collect(),
    }
}

// Borrow protocol fields while preserving the HostValue::Record wire format
// and budgets, without cloning payloads into an owned value tree.
pub(crate) fn encode_record_to(
    fields: &[(&str, RecordValueRef<'_>)],
    limits: &StorageLimits,
    writer: impl Write,
) -> Result<usize, HostError> {
    use RecordValueRef::*;
    let mut budget = limits.value_budget();
    let node = std::mem::size_of::<HostValue>();
    budget.add(0, node)?;
    let mut names = std::collections::BTreeSet::new();
    for (name, value) in fields {
        if !names.insert(name) {
            return Err(schema("duplicate stored record field"));
        }
        budget.add(0, name.len())?;
        match value {
            String(s) => {
                budget.add(1, node)?;
                budget.add(1, s.len())?;
            }
            Value(value) => budget.host(value, 1)?,
            OptionalString(value) => {
                budget.add(1, node)?;
                budget.add(1, 4)?;
                if let Some(s) = value {
                    budget.add(2, node)?;
                    budget.add(2, s.len())?;
                }
            }
            OptionalValue(value) => {
                budget.add(1, node)?;
                budget.add(1, 4)?;
                if let Some(value) = value {
                    budget.host(value, 2)?;
                }
            }
        }
    }
    let mut out = Output::new(writer, limits.max_value_bytes);
    out.raw("{\"fields\":[")?;
    for (index, (name, value)) in fields.iter().enumerate() {
        if index > 0 {
            out.raw(",")?;
        }
        out.raw("{\"name\":")?;
        out.json(name)?;
        out.raw(",\"value\":")?;
        match value {
            String(s) => out.scalar("string", s)?,
            Value(value) => out.host(value)?,
            OptionalString(value) => option(&mut out, *value, |out, s| out.scalar("string", s))?,
            OptionalValue(value) => option(&mut out, *value, |out, value| out.host(value))?,
        }
        out.raw("}")?;
    }
    out.raw("],\"kind\":\"record\"}")?;
    Ok(out.used)
}

fn option<W: Write, T>(
    out: &mut Output<W>,
    value: Option<T>,
    write: impl FnOnce(&mut Output<W>, T) -> Result<(), HostError>,
) -> Result<(), HostError> {
    out.raw("{\"fields\":[")?;
    let name = if let Some(value) = value {
        write(out, value)?;
        "Some"
    } else {
        "None"
    };
    out.raw("],\"kind\":\"variant\",\"name\":")?;
    out.json(name)?;
    out.raw("}")
}
