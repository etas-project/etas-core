use super::{budget::MetadataGraphBudget, descriptors, schema::ProtoParameterDefault};
use crate::{MetadataArtifactError, ParameterDefault};

pub(super) fn encode(value: &ParameterDefault) -> ProtoParameterDefault {
    ProtoParameterDefault {
        slot: value.slot,
        reference: Some(descriptors::encode_parameter_default(&value.reference)),
        callable_value: value
            .callable_value
            .as_ref()
            .map(super::codec::callable_values::to_proto),
    }
}

pub(super) fn decode(
    value: ProtoParameterDefault,
) -> Result<ParameterDefault, MetadataArtifactError> {
    Ok(ParameterDefault {
        slot: value.slot,
        callable_value: value
            .callable_value
            .map(super::codec::callable_values::from_proto)
            .transpose()?,
        reference: descriptors::decode_parameter_default(
            value
                .reference
                .ok_or_else(|| invalid("default slot has no canonical expression reference"))?,
        )?,
    })
}

pub(super) fn validate_model(
    values: &[ParameterDefault],
    arity: usize,
    budget: &mut MetadataGraphBudget,
) -> Result<(), MetadataArtifactError> {
    validate_slots(values.iter().map(|value| value.slot), arity)?;
    budget.charge(values.len(), "declaration default slots")?;
    for value in values {
        value
            .reference
            .owner
            .validate()
            .map_err(|error| invalid(error.to_string()))?;
        descriptors::validate_parameter_default(
            &descriptors::encode_parameter_default(&value.reference),
            budget,
        )?;
        if let Some(graph) = &value.callable_value {
            budget.charge(
                super::budget::validate_callable_values(graph)?,
                "default callable value",
            )?;
        }
    }
    Ok(())
}

pub(super) fn validate_wire(
    values: &[ProtoParameterDefault],
    arity: usize,
    budget: &mut MetadataGraphBudget,
) -> Result<(), MetadataArtifactError> {
    validate_slots(values.iter().map(|value| value.slot), arity)?;
    budget.charge(values.len(), "declaration default slots")?;
    for value in values {
        descriptors::validate_parameter_default(
            value
                .reference
                .as_ref()
                .ok_or_else(|| invalid("default slot has no canonical expression reference"))?,
            budget,
        )?;
        if let Some(graph) = &value.callable_value {
            super::budget::validate_proto_callable_values(graph, budget)?;
        }
    }
    Ok(())
}

fn validate_slots(
    slots: impl Iterator<Item = u32>,
    arity: usize,
) -> Result<(), MetadataArtifactError> {
    let mut previous = None;
    for slot in slots {
        if slot as usize >= arity || previous.is_some_and(|previous| previous >= slot) {
            return Err(invalid(
                "declaration default slots must be distinct, ordered parameter positions",
            ));
        }
        previous = Some(slot);
    }
    Ok(())
}

fn invalid(message: impl Into<String>) -> MetadataArtifactError {
    MetadataArtifactError::invalid(crate::PACKAGE_METADATA_FILE, message)
}
