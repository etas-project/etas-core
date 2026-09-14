use super::*;

pub fn project_to_host_value(value: &impl HostValueProjection) -> Result<HostValue, HostError> {
    value.project(ValueBuilder)
}

struct ValueBuilder;

impl HostValueVisitor for ValueBuilder {
    type Output = HostValue;

    fn scalar(self, value: HostScalar<'_>) -> Result<Self::Output, HostError> {
        Ok(match value {
            HostScalar::Unit => HostValue::Unit,
            HostScalar::Bool(v) => HostValue::Bool(v),
            HostScalar::Int(v) => HostValue::Int(v),
            HostScalar::UInt(v) => HostValue::UInt(v),
            HostScalar::Float(v) => HostValue::Float(v),
            HostScalar::String(v) => HostValue::String(v.to_owned()),
            HostScalar::Bytes(v) => HostValue::Bytes(v.to_owned()),
        })
    }

    fn list<P: HostValueProjection>(
        self,
        values: impl IntoIterator<Item = P>,
    ) -> Result<Self::Output, HostError> {
        values
            .into_iter()
            .map(|v| v.project(Self))
            .collect::<Result<_, _>>()
            .map(HostValue::List)
    }

    fn map<P: HostValueProjection>(
        self,
        entries: impl IntoIterator<Item = (P, P)>,
    ) -> Result<Self::Output, HostError> {
        entries
            .into_iter()
            .map(|(k, v)| Ok((k.project(Self)?, v.project(Self)?)))
            .collect::<Result<_, _>>()
            .map(HostValue::Map)
    }

    fn record<'a, P: HostValueProjection>(
        self,
        fields: impl IntoIterator<Item = (&'a str, P)>,
    ) -> Result<Self::Output, HostError> {
        fields
            .into_iter()
            .map(|(k, v)| Ok((k.to_owned(), v.project(Self)?)))
            .collect::<Result<_, _>>()
            .map(HostValue::Record)
    }

    fn variant<P: HostValueProjection>(
        self,
        name: &str,
        fields: impl IntoIterator<Item = P>,
    ) -> Result<Self::Output, HostError> {
        Ok(HostValue::Variant {
            name: name.to_owned(),
            fields: fields
                .into_iter()
                .map(|v| v.project(Self))
                .collect::<Result<_, _>>()?,
        })
    }

    fn json<P: HostJsonProjection>(self, value: &P) -> Result<Self::Output, HostError> {
        value.project_json(JsonBuilder).map(HostValue::Json)
    }
}

struct JsonBuilder;

impl HostJsonVisitor for JsonBuilder {
    type Output = HostJsonValue;
    fn null(self) -> Result<Self::Output, HostError> {
        Ok(HostJsonValue::Null)
    }
    fn boolean(self, value: bool) -> Result<Self::Output, HostError> {
        Ok(HostJsonValue::Bool(value))
    }
    fn number(self, value: f64) -> Result<Self::Output, HostError> {
        Ok(HostJsonValue::Number(value))
    }
    fn string(self, value: &str) -> Result<Self::Output, HostError> {
        Ok(HostJsonValue::String(value.to_owned()))
    }
    fn array<P: HostJsonProjection>(
        self,
        values: impl IntoIterator<Item = P>,
    ) -> Result<Self::Output, HostError> {
        values
            .into_iter()
            .map(|v| v.project_json(Self))
            .collect::<Result<_, _>>()
            .map(HostJsonValue::Array)
    }
    fn object<'a, P: HostJsonProjection>(
        self,
        fields: impl IntoIterator<Item = (&'a str, P)>,
    ) -> Result<Self::Output, HostError> {
        fields
            .into_iter()
            .map(|(k, v)| Ok((k.to_owned(), v.project_json(Self)?)))
            .collect::<Result<_, _>>()
            .map(HostJsonValue::Object)
    }
}
