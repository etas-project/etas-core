use super::{budget::MetadataGraphBudget, descriptors, schema::ProtoParameterDefault};
use crate::{MetadataArtifactError, ParameterDefault};

pub(super) fn encode(value: &ParameterDefault) -> ProtoParameterDefault {
    ProtoParameterDefault {
        binding: value
            .binding
            .as_ref()
            .map(|binding| super::codec::effect_summary_to_proto(&binding.as_effect_summary())),
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
        binding: value
            .binding
            .map(super::codec::effect_summary_from_proto)
            .transpose()?
            .map(|summary| {
                Ok(crate::ParameterBindingSummary {
                    behavior: summary.behavior.ok_or_else(|| {
                        invalid("injected binding is missing its checked behavior")
                    })?,
                    public_effects: summary.public_effects,
                    requested_actions: summary.requested_actions,
                    handled_requested_actions: summary.handled_requested_actions,
                    default_handler_effects: summary.default_handler_effects.ok_or_else(|| {
                        invalid("injected binding is missing its default-handler summary")
                    })?,
                    action_trace: summary.action_trace,
                    handler_requests: summary.handler_requests,
                })
            })
            .transpose()?,
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
        if value.reference.injected != value.binding.is_some() {
            return Err(invalid(
                "injected parameters require their conditional binding summary",
            ));
        }
        if let Some(binding) = &value.binding {
            super::budget::validate_parameter_binding(binding, budget)?;
        }
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
        if value
            .reference
            .as_ref()
            .is_some_and(|reference| reference.injected)
            != value.binding.is_some()
        {
            return Err(invalid(
                "injected parameters require their conditional binding summary",
            ));
        }
        if let Some(binding) = &value.binding {
            super::budget::validate_proto_parameter_binding(binding, budget)?;
        }
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
