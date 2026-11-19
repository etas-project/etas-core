use super::*;

pub fn project_to_host_value(value: &impl HostValueProjection) -> Result<HostValue, HostError> {
    value.project(ValueBuilder {
        budget: None,
        depth: 0,
    })
}

/// Charge the neutral value budget before allocating projected fields/payloads.
pub fn project_to_host_value_bounded(
    value: &impl HostValueProjection,
    budget: &mut crate::ValueBudget<'_>,
) -> Result<HostValue, HostError> {
    value.project(ValueBuilder {
        budget: Some(budget),
        depth: 0,
    })
}

struct ValueBuilder<'a, 'limits> {
    budget: Option<&'a mut crate::ValueBudget<'limits>>,
    depth: usize,
}

impl<'limits> ValueBuilder<'_, 'limits> {
    fn charge(&mut self, bytes: usize) -> Result<(), HostError> {
        if let Some(budget) = &mut self.budget {
            budget.add(self.depth, bytes)?;
        }
        Ok(())
    }

    fn child(&mut self) -> ValueBuilder<'_, 'limits> {
        ValueBuilder {
            budget: self.budget.as_deref_mut(),
            depth: self.depth + 1,
        }
    }
}

impl HostValueVisitor for ValueBuilder<'_, '_> {
    type Output = HostValue;

    fn text<'a>(
        mut self,
        byte_len: usize,
        chunks: impl IntoIterator<Item = &'a str>,
    ) -> Result<Self::Output, HostError> {
        self.charge(std::mem::size_of::<HostValue>())?;
        self.charge(byte_len)?;
        text::materialize(byte_len, chunks).map(HostValue::String)
    }

    fn scalar(mut self, value: HostScalar<'_>) -> Result<Self::Output, HostError> {
        self.charge(std::mem::size_of::<HostValue>())?;
        match &value {
            HostScalar::String(value) => self.charge(value.len())?,
            HostScalar::Bytes(value) => self.charge(value.len())?,
            _ => {}
        }
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
        mut self,
        values: impl IntoIterator<Item = P>,
    ) -> Result<Self::Output, HostError> {
        self.charge(std::mem::size_of::<HostValue>())?;
        let mut output = Vec::new();
        for value in values {
            output.push(value.project(self.child())?);
        }
        Ok(HostValue::List(output))
    }

    fn map<P: HostValueProjection>(
        mut self,
        entries: impl IntoIterator<Item = (P, P)>,
    ) -> Result<Self::Output, HostError> {
        self.charge(std::mem::size_of::<HostValue>())?;
        let mut output = Vec::new();
        for (key, value) in entries {
            output.push((key.project(self.child())?, value.project(self.child())?));
        }
        Ok(HostValue::Map(output))
    }

    fn record<'a, P: HostValueProjection>(
        mut self,
        fields: impl IntoIterator<Item = (&'a str, P)>,
    ) -> Result<Self::Output, HostError> {
        self.charge(std::mem::size_of::<HostValue>())?;
        let mut output = Vec::new();
        for (name, value) in fields {
            self.charge(name.len())?;
            output.push((name.to_owned(), value.project(self.child())?));
        }
        Ok(HostValue::Record(output))
    }

    fn variant<P: HostValueProjection>(
        mut self,
        name: &str,
        fields: impl IntoIterator<Item = P>,
    ) -> Result<Self::Output, HostError> {
        self.charge(std::mem::size_of::<HostValue>())?;
        self.charge(name.len())?;
        let mut output = Vec::new();
        for value in fields {
            output.push(value.project(self.child())?);
        }
        Ok(HostValue::Variant {
            name: name.to_owned(),
            fields: output,
        })
    }

    fn json<P: HostJsonProjection>(mut self, value: &P) -> Result<Self::Output, HostError> {
        self.charge(std::mem::size_of::<HostValue>())?;
        value
            .project_json(JsonBuilder(self.child()))
            .map(HostValue::Json)
    }
}

struct JsonBuilder<'a, 'limits>(ValueBuilder<'a, 'limits>);

impl HostJsonVisitor for JsonBuilder<'_, '_> {
    type Output = HostJsonValue;
    fn text<'a>(
        mut self,
        byte_len: usize,
        chunks: impl IntoIterator<Item = &'a str>,
    ) -> Result<Self::Output, HostError> {
        self.0.charge(std::mem::size_of::<HostJsonValue>())?;
        self.0.charge(byte_len)?;
        text::materialize(byte_len, chunks).map(HostJsonValue::String)
    }
    fn null(mut self) -> Result<Self::Output, HostError> {
        self.0.charge(std::mem::size_of::<HostJsonValue>())?;
        Ok(HostJsonValue::Null)
    }
    fn boolean(mut self, value: bool) -> Result<Self::Output, HostError> {
        self.0.charge(std::mem::size_of::<HostJsonValue>())?;
        Ok(HostJsonValue::Bool(value))
    }
    fn number(mut self, value: f64) -> Result<Self::Output, HostError> {
        self.0.charge(std::mem::size_of::<HostJsonValue>())?;
        Ok(HostJsonValue::Number(value))
    }
    fn string(mut self, value: &str) -> Result<Self::Output, HostError> {
        self.0.charge(std::mem::size_of::<HostJsonValue>())?;
        self.0.charge(value.len())?;
        Ok(HostJsonValue::String(value.to_owned()))
    }
    fn array<P: HostJsonProjection>(
        mut self,
        values: impl IntoIterator<Item = P>,
    ) -> Result<Self::Output, HostError> {
        self.0.charge(std::mem::size_of::<HostJsonValue>())?;
        let mut output = Vec::new();
        for value in values {
            output.push(value.project_json(JsonBuilder(self.0.child()))?);
        }
        Ok(HostJsonValue::Array(output))
    }
    fn object<'a, P: HostJsonProjection>(
        mut self,
        fields: impl IntoIterator<Item = (&'a str, P)>,
    ) -> Result<Self::Output, HostError> {
        self.0.charge(std::mem::size_of::<HostJsonValue>())?;
        let mut output = Vec::new();
        for (name, value) in fields {
            self.0.charge(name.len())?;
            output.push((
                name.to_owned(),
                value.project_json(JsonBuilder(self.0.child()))?,
            ));
        }
        Ok(HostJsonValue::Object(output))
    }
}
