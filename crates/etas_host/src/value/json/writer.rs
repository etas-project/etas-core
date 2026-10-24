use serde::Serialize;

use super::{HostError, HostErrorCode, HostValue, json_number_error};
use crate::value::projection::{
    HostJsonProjection, HostJsonVisitor, HostScalar, HostValueProjection, HostValueVisitor,
};

/// Encode the canonical JSON projection without an intermediate value graph.
pub fn host_value_to_json_string(value: &HostValue) -> Result<String, HostError> {
    project_to_json_string(value)
}

pub fn project_to_json_string(value: &impl HostValueProjection) -> Result<String, HostError> {
    let mut writer = JsonWriter { output: Vec::new() };
    value.project(&mut writer)?;
    String::from_utf8(writer.output).map_err(|error| json_number_error(error.to_string()))
}

struct JsonWriter {
    output: Vec<u8>,
}

impl JsonWriter {
    fn scalar(&mut self, value: &(impl Serialize + ?Sized)) -> Result<(), HostError> {
        serde_json::to_writer(&mut self.output, value)
            .map_err(|error| json_number_error(error.to_string()))
    }

    fn array<P>(
        &mut self,
        values: impl IntoIterator<Item = P>,
        mut encode: impl FnMut(&mut Self, P) -> Result<(), HostError>,
    ) -> Result<(), HostError> {
        self.output.push(b'[');
        for (index, value) in values.into_iter().enumerate() {
            if index != 0 {
                self.output.push(b',');
            }
            encode(self, value)?;
        }
        self.output.push(b']');
        Ok(())
    }

    fn object<'a, P>(
        &mut self,
        fields: impl IntoIterator<Item = (&'a str, P)>,
        duplicate_error: &str,
        mut encode: impl FnMut(&mut Self, P) -> Result<(), HostError>,
    ) -> Result<(), HostError> {
        let mut fields: Vec<_> = fields.into_iter().collect();
        fields.sort_unstable_by(|left, right| left.0.cmp(right.0));
        for pair in fields.windows(2) {
            if pair[0].0 == pair[1].0 {
                return Err(
                    HostError::new(HostErrorCode::SchemaMismatch, duplicate_error)
                        .with_detail("field", pair[0].0),
                );
            }
        }
        self.output.push(b'{');
        for (index, (name, value)) in fields.into_iter().enumerate() {
            if index != 0 {
                self.output.push(b',');
            }
            self.scalar(name)?;
            self.output.push(b':');
            encode(self, value)?;
        }
        self.output.push(b'}');
        Ok(())
    }
}

impl HostValueVisitor for &mut JsonWriter {
    type Output = ();

    fn scalar(self, value: HostScalar<'_>) -> Result<(), HostError> {
        match value {
            HostScalar::Unit => JsonWriter::scalar(self, &()),
            HostScalar::Bool(v) => JsonWriter::scalar(self, &v),
            HostScalar::Int(v) => {
                let v = i64::try_from(v)
                    .map_err(|_| json_number_error("signed integer is outside JSON i64 range"))?;
                JsonWriter::scalar(self, &v)
            }
            HostScalar::UInt(v) => {
                let v = u64::try_from(v)
                    .map_err(|_| json_number_error("unsigned integer is outside JSON u64 range"))?;
                JsonWriter::scalar(self, &v)
            }
            HostScalar::Float(v) => {
                let v = serde_json::Number::from_f64(v)
                    .ok_or_else(|| json_number_error("floating-point value is not finite"))?;
                JsonWriter::scalar(self, &v)
            }
            HostScalar::String(v) => JsonWriter::scalar(self, v),
            HostScalar::Bytes(v) => JsonWriter::scalar(self, v),
        }
    }

    fn list<P: HostValueProjection>(
        self,
        values: impl IntoIterator<Item = P>,
    ) -> Result<(), HostError> {
        self.array(values, |writer, value| value.project(writer))
    }

    fn map<P: HostValueProjection>(
        self,
        entries: impl IntoIterator<Item = (P, P)>,
    ) -> Result<(), HostError> {
        self.array(entries, |writer, (key, value)| writer.list([key, value]))
    }

    fn record<'a, P: HostValueProjection>(
        self,
        fields: impl IntoIterator<Item = (&'a str, P)>,
    ) -> Result<(), HostError> {
        self.object(
            fields,
            "record contains duplicate JSON field name",
            |writer, value| value.project(writer),
        )
    }

    fn variant<P: HostValueProjection>(
        self,
        name: &str,
        fields: impl IntoIterator<Item = P>,
    ) -> Result<(), HostError> {
        self.output.extend_from_slice(b"{\"fields\":");
        self.array(fields, |writer, value| value.project(writer))?;
        self.output.extend_from_slice(b",\"name\":");
        JsonWriter::scalar(self, name)?;
        self.output.push(b'}');
        Ok(())
    }

    fn json<P: HostJsonProjection>(self, value: &P) -> Result<(), HostError> {
        value.project_json(self)
    }
}

impl HostJsonVisitor for &mut JsonWriter {
    type Output = ();
    fn null(self) -> Result<(), HostError> {
        JsonWriter::scalar(self, &())
    }
    fn boolean(self, value: bool) -> Result<(), HostError> {
        JsonWriter::scalar(self, &value)
    }
    fn number(self, value: f64) -> Result<(), HostError> {
        let value = serde_json::Number::from_f64(value)
            .ok_or_else(|| json_number_error("host JSON number is not finite"))?;
        JsonWriter::scalar(self, &value)
    }
    fn string(self, value: &str) -> Result<(), HostError> {
        JsonWriter::scalar(self, value)
    }
    fn array<P: HostJsonProjection>(
        self,
        values: impl IntoIterator<Item = P>,
    ) -> Result<(), HostError> {
        JsonWriter::array(self, values, |writer, value| value.project_json(writer))
    }
    fn object<'a, P: HostJsonProjection>(
        self,
        fields: impl IntoIterator<Item = (&'a str, P)>,
    ) -> Result<(), HostError> {
        JsonWriter::object(
            self,
            fields,
            "host JSON object contains duplicate field name",
            |writer, value| value.project_json(writer),
        )
    }
}

#[cfg(test)]
mod tests;
