use super::{budget::MetadataGraphBudget, descriptors, schema::ProtoParameterDefault};
use crate::{MetadataArtifactError, ParameterDefault};

pub(super) fn encode(value: &ParameterDefault) -> ProtoParameterDefault {
    ProtoParameterDefault {
        slot: value.slot,
        reference: Some(descriptors::encode_parameter_default(&value.reference)),
    }
}

pub(super) fn decode(
    value: ProtoParameterDefault,
) -> Result<ParameterDefault, MetadataArtifactError> {
    Ok(ParameterDefault {
        slot: value.slot,
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
