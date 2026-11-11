use super::*;

pub(super) fn to_proto(method: &PublishedMethod) -> ProtoPublishedMethod {
    let (receiver, receiver_slot) = match method.receiver {
        MethodReceiver::Value { slot } => ("value", Some(slot)),
        MethodReceiver::Associated => ("associated", None),
        MethodReceiver::Component => ("component", None),
    };
    ProtoPublishedMethod {
        identity: Some(super::super::descriptors::encode_method_identity(
            &method.identity,
        )),
        self_type: Some(type_to_proto(&method.self_type)),
        receiver: receiver.into(),
        receiver_slot,
        owner_params: method
            .owner_params
            .iter()
            .map(generic_param_to_proto)
            .collect(),
        callable: Some(callable_values::to_proto(&method.callable)),
        limits: method
            .requirements
            .limits
            .iter()
            .map(limit_to_proto)
            .collect(),
        trace_requirements: method
            .requirements
            .trace_requirements
            .iter()
            .map(trace_to_proto)
            .collect(),
        trace_obligations: method
            .requirements
            .trace_obligations
            .iter()
            .map(trace_to_proto)
            .collect(),
    }
}

pub(super) fn from_proto(
    method: ProtoPublishedMethod,
) -> Result<PublishedMethod, MetadataArtifactError> {
    let receiver = match (method.receiver.as_str(), method.receiver_slot) {
        ("value", Some(slot)) => MethodReceiver::Value { slot },
        ("associated", None) => MethodReceiver::Associated,
        ("component", None) => MethodReceiver::Component,
        _ => return Err(invalid_method("invalid published method receiver ABI")),
    };
    let method = PublishedMethod {
        identity: super::super::descriptors::decode_method_identity(required_message(
            method.identity,
            "published method identity",
        )?)?,
        self_type: type_from_proto(required_message(
            method.self_type,
            "published method owner type",
        )?)?,
        receiver,
        requirements: MethodRequirements {
            limits: method
                .limits
                .into_iter()
                .map(limit_from_proto)
                .collect::<Result<_, _>>()?,
            trace_requirements: method
                .trace_requirements
                .into_iter()
                .map(trace_from_proto)
                .collect::<Result<_, _>>()?,
            trace_obligations: method
                .trace_obligations
                .into_iter()
                .map(trace_from_proto)
                .collect::<Result<_, _>>()?,
        },
        owner_params: method
            .owner_params
            .into_iter()
            .map(generic_param_from_proto)
            .collect::<Result<_, _>>()?,
        callable: callable_values::from_proto(required_message(
            method.callable,
            "published method behavior graph",
        )?)?,
    };
    method.validate()?;
    Ok(method)
}

pub(super) fn trace_to_proto(requirement: &MethodTraceRequirement) -> ProtoMethodTraceRequirement {
    match requirement {
        MethodTraceRequirement::Clause(clause) => ProtoMethodTraceRequirement {
            clause: Some(trace_spec_clause_to_proto(clause)),
            reference: None,
            limit: None,
        },
        MethodTraceRequirement::Reference(name) => ProtoMethodTraceRequirement {
            clause: None,
            reference: Some(name.clone()),
            limit: None,
        },
        MethodTraceRequirement::Limit(count) => ProtoMethodTraceRequirement {
            clause: None,
            reference: None,
            limit: Some(*count),
        },
    }
}

pub(super) fn trace_from_proto(
    requirement: ProtoMethodTraceRequirement,
) -> Result<MethodTraceRequirement, MetadataArtifactError> {
    match (requirement.clause, requirement.reference, requirement.limit) {
        (Some(clause), None, None) => Ok(MethodTraceRequirement::Clause(
            trace_spec_clause_from_proto(clause)?,
        )),
        (None, Some(name), None) if !name.is_empty() => Ok(MethodTraceRequirement::Reference(name)),
        (None, None, Some(count)) => Ok(MethodTraceRequirement::Limit(count)),
        _ => Err(invalid_method("invalid method trace requirement")),
    }
}

pub(super) fn limit_to_proto(limit: &MethodLimitRequirement) -> ProtoMethodLimitRequirement {
    let (value_kind, amount, currency) = match &limit.value {
        None => ("", String::new(), String::new()),
        Some(MethodLimitValue::Count(value)) => ("count", value.to_string(), String::new()),
        Some(MethodLimitValue::DurationMillis(value)) => {
            ("duration", value.to_string(), String::new())
        }
        Some(MethodLimitValue::MoneyMicros { amount, currency }) => {
            ("money", amount.to_string(), currency.clone())
        }
    };
    ProtoMethodLimitRequirement {
        kind: format!("{:?}", limit.kind),
        value_kind: value_kind.into(),
        amount,
        currency,
    }
}

pub(super) fn limit_from_proto(
    limit: ProtoMethodLimitRequirement,
) -> Result<MethodLimitRequirement, MetadataArtifactError> {
    let kind = match limit.kind.as_str() {
        "Iterations" => MethodLimitKind::Iterations,
        "Tokens" => MethodLimitKind::Tokens,
        "ContextTokens" => MethodLimitKind::ContextTokens,
        "Cost" => MethodLimitKind::Cost,
        "WallTime" => MethodLimitKind::WallTime,
        "Attempts" => MethodLimitKind::Attempts,
        "Concurrency" => MethodLimitKind::Concurrency,
        _ => return Err(invalid_method("unknown method limit kind")),
    };
    let value = match (limit.value_kind.as_str(), limit.currency.as_str()) {
        ("", "") if limit.amount.is_empty() => None,
        ("count", "") => Some(MethodLimitValue::Count(
            limit
                .amount
                .parse()
                .map_err(|_| invalid_method("invalid method count limit"))?,
        )),
        ("duration", "") => Some(MethodLimitValue::DurationMillis(
            limit
                .amount
                .parse()
                .map_err(|_| invalid_method("invalid method duration limit"))?,
        )),
        ("money", currency) if !currency.is_empty() => Some(MethodLimitValue::MoneyMicros {
            amount: limit
                .amount
                .parse()
                .map_err(|_| invalid_method("invalid method money limit"))?,
            currency: currency.into(),
        }),
        _ => return Err(invalid_method("invalid method limit value")),
    };
    Ok(MethodLimitRequirement { kind, value })
}

fn invalid_method(reason: &str) -> MetadataArtifactError {
    MetadataArtifactError::invalid(crate::PACKAGE_METADATA_FILE, reason)
}
