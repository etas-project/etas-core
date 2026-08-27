mod control;
mod io;
mod model;
mod state;

use crate::{
    HostFieldSchema, HostSchema, HostTracePayload, HostValue, HostVariantSchema, ToolRef,
    ToolSchema,
};

pub trait HostTraceRequest {
    fn trace_payload(&self) -> HostTracePayload;
}

fn record(fields: impl IntoIterator<Item = (&'static str, HostValue)>) -> HostValue {
    HostValue::Record(
        fields
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
    )
}

fn variant(name: &str, fields: Vec<HostValue>) -> HostValue {
    HostValue::Variant {
        name: name.to_owned(),
        fields,
    }
}

fn option(value: Option<HostValue>) -> HostValue {
    match value {
        Some(value) => variant("Some", vec![value]),
        None => variant("None", Vec::new()),
    }
}

fn strings(values: &[String]) -> HostValue {
    HostValue::List(values.iter().cloned().map(HostValue::String).collect())
}

fn schema(value: &HostSchema) -> HostValue {
    match value {
        HostSchema::Unit => variant("Unit", Vec::new()),
        HostSchema::Bool => variant("Bool", Vec::new()),
        HostSchema::Int => variant("Int", Vec::new()),
        HostSchema::UInt => variant("UInt", Vec::new()),
        HostSchema::Float => variant("Float", Vec::new()),
        HostSchema::String => variant("String", Vec::new()),
        HostSchema::Bytes => variant("Bytes", Vec::new()),
        HostSchema::List(element) => variant("List", vec![schema(element)]),
        HostSchema::Map { key, value } => variant("Map", vec![schema(key), schema(value)]),
        HostSchema::Record(fields) => variant(
            "Record",
            vec![HostValue::List(fields.iter().map(field_schema).collect())],
        ),
        HostSchema::Variant(variants) => variant(
            "Variant",
            vec![HostValue::List(
                variants.iter().map(variant_schema).collect(),
            )],
        ),
        HostSchema::Json => variant("Json", Vec::new()),
    }
}

fn field_schema(field: &HostFieldSchema) -> HostValue {
    record([
        ("name", HostValue::String(field.name.clone())),
        ("schema", schema(&field.schema)),
        ("optional", HostValue::Bool(field.optional)),
    ])
}

fn variant_schema(item: &HostVariantSchema) -> HostValue {
    record([
        ("name", HostValue::String(item.name.clone())),
        (
            "fields",
            HostValue::List(item.fields.iter().map(schema).collect()),
        ),
    ])
}

fn tool_ref(tool: &ToolRef) -> HostValue {
    record([
        ("name", HostValue::String(tool.name.clone())),
        (
            "qualified_name",
            option(tool.qualified_name.clone().map(HostValue::String)),
        ),
        (
            "std_symbol",
            option(
                tool.std_symbol
                    .map(|symbol| HostValue::UInt(symbol.0 as u128)),
            ),
        ),
    ])
}

fn tool_schema(tool: &ToolSchema) -> HostValue {
    record([
        ("tool", tool_ref(&tool.tool)),
        ("input", schema(&tool.input)),
        ("output", option(tool.output.as_ref().map(schema))),
    ])
}
