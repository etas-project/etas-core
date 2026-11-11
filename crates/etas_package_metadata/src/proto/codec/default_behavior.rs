use super::*;

pub(super) fn to_proto(value: &DefaultBehavior) -> ProtoDefaultBehavior {
    ProtoDefaultBehavior {
        direct: Some(footprint_to_proto(&value.direct)),
        guarded: value
            .guarded
            .iter()
            .map(|entry| ProtoGuardedEffectFootprint {
                guards: entry.guards.iter().map(effect_ref_to_proto).collect(),
                footprint: Some(footprint_to_proto(&entry.footprint)),
            })
            .collect(),
    }
}

pub(super) fn from_proto(
    value: ProtoDefaultBehavior,
) -> Result<DefaultBehavior, MetadataArtifactError> {
    Ok(DefaultBehavior {
        direct: footprint_from_proto(
            value
                .direct
                .ok_or_else(|| invalid("default behavior is missing its direct footprint"))?,
        )?,
        guarded: value
            .guarded
            .into_iter()
            .map(|entry| {
                if entry.guards.is_empty() {
                    return Err(invalid("default behavior contribution has no guards"));
                }
                Ok(GuardedEffectFootprint {
                    guards: entry
                        .guards
                        .into_iter()
                        .map(effect_ref_from_proto)
                        .collect::<Result<_, _>>()?,
                    footprint: footprint_from_proto(entry.footprint.ok_or_else(|| {
                        invalid("default behavior contribution has no footprint")
                    })?)?,
                })
            })
            .collect::<Result<_, MetadataArtifactError>>()?,
    })
}

fn footprint_to_proto(value: &EffectFootprint) -> ProtoEffectFootprint {
    ProtoEffectFootprint {
        escaping_effects: Some(effect_row_to_proto(&value.escaping_effects)),
        requested_actions: Some(effect_row_to_proto(&value.requested_actions)),
        default_actions: Some(effect_row_to_proto(&value.default_actions)),
        handled_actions: Some(effect_row_to_proto(&value.handled_actions)),
        handler_requests: value
            .handler_requests
            .as_ref()
            .map(request_template_to_proto),
        residual_checks: Some(callable_values::residual_to_proto(&value.residual_checks)),
        behavior: Some(callable_values::behavior_to_proto(&value.behavior)),
        limits: value
            .requirements
            .limits
            .iter()
            .map(methods::limit_to_proto)
            .collect(),
        trace_requirements: value
            .requirements
            .trace_requirements
            .iter()
            .map(methods::trace_to_proto)
            .collect(),
        trace_obligations: value
            .requirements
            .trace_obligations
            .iter()
            .map(methods::trace_to_proto)
            .collect(),
    }
}

fn footprint_from_proto(
    value: ProtoEffectFootprint,
) -> Result<EffectFootprint, MetadataArtifactError> {
    Ok(EffectFootprint {
        escaping_effects: effect_row_from_proto(
            value
                .escaping_effects
                .ok_or_else(|| invalid("effect footprint has no escaping row"))?,
        )?,
        requested_actions: effect_row_from_proto(
            value
                .requested_actions
                .ok_or_else(|| invalid("effect footprint has no requested row"))?,
        )?,
        default_actions: effect_row_from_proto(
            value
                .default_actions
                .ok_or_else(|| invalid("effect footprint has no default row"))?,
        )?,
        handled_actions: effect_row_from_proto(
            value
                .handled_actions
                .ok_or_else(|| invalid("effect footprint has no handled row"))?,
        )?,
        handler_requests: value
            .handler_requests
            .map(request_template_from_proto)
            .transpose()?,
        residual_checks: callable_values::residual_from_proto(
            value
                .residual_checks
                .ok_or_else(|| invalid("effect footprint has no residual references"))?,
        )?,
        behavior: callable_values::behavior_from_proto(
            value
                .behavior
                .ok_or_else(|| invalid("effect footprint has no behavior"))?,
        )?,
        requirements: MethodRequirements {
            limits: value
                .limits
                .into_iter()
                .map(methods::limit_from_proto)
                .collect::<Result<_, _>>()?,
            trace_requirements: value
                .trace_requirements
                .into_iter()
                .map(methods::trace_from_proto)
                .collect::<Result<_, _>>()?,
            trace_obligations: value
                .trace_obligations
                .into_iter()
                .map(methods::trace_from_proto)
                .collect::<Result<_, _>>()?,
        },
    })
}
