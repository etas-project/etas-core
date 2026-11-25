use super::descriptors::ProtoValueProjectionStep as P;
use crate::MetadataArtifactError;
use etas_code::descriptor::ValueProjectionStep as S;

pub(super) fn encode(step: &S) -> P {
    let (kind, name, position) = match step {
        S::Field(name) => (0, name.clone(), 0),
        S::TupleElement(position) => (1, String::new(), *position),
        S::VariantPayload { variant, position } => (2, variant.clone(), *position),
        S::SequenceElement => (3, String::new(), 0),
        S::ListHead => (4, String::new(), 0),
        S::ListTail => (5, String::new(), 0),
        S::MapKey => (6, String::new(), 0),
        S::MapValue => (7, String::new(), 0),
        S::CallResult => (8, String::new(), 0),
        S::CallWithArguments { inputs } => (10, String::new(), *inputs),
        S::Invocation { inputs } => (11, String::new(), *inputs),
        S::Repeat { .. } => (9, String::new(), 0),
    };
    P {
        kind,
        name,
        position,
        alternatives: match step {
            S::Repeat { alternatives } => alternatives
                .iter()
                .map(|path| super::descriptors::ProtoValueProjectionPath {
                    steps: path.iter().map(encode).collect(),
                })
                .collect(),
            _ => Vec::new(),
        },
    }
}

pub(super) fn decode(step: P) -> Result<S, MetadataArtifactError> {
    if step.kind == 9 && step.name.is_empty() && step.position == 0 {
        let result = S::Repeat {
            alternatives: step
                .alternatives
                .into_iter()
                .map(|path| path.steps.into_iter().map(decode).collect())
                .collect::<Result<_, _>>()?,
        };
        if S::well_formed(
            std::slice::from_ref(&result),
            crate::MAX_METADATA_GRAPH_DEPTH,
            crate::MAX_METADATA_GRAPH_NODES,
        ) {
            return Ok(result);
        }
        return Err(MetadataArtifactError::invalid(
            crate::PACKAGE_METADATA_FILE,
            "invalid repeated value projection",
        ));
    }
    if !step.alternatives.is_empty() {
        return Err(MetadataArtifactError::invalid(
            crate::PACKAGE_METADATA_FILE,
            "unexpected repeated value projection",
        ));
    }
    Ok(match (step.kind, step.name.as_str(), step.position) {
        (0, name, 0) if !name.is_empty() => S::Field(step.name),
        (1, "", position) => S::TupleElement(position),
        (2, name, position) if !name.is_empty() => S::VariantPayload {
            variant: step.name,
            position,
        },
        (3, "", 0) => S::SequenceElement,
        (4, "", 0) => S::ListHead,
        (5, "", 0) => S::ListTail,
        (6, "", 0) => S::MapKey,
        (7, "", 0) => S::MapValue,
        (8, "", 0) => S::CallResult,
        (10, "", inputs) => S::CallWithArguments { inputs },
        (11, "", inputs) => S::Invocation { inputs },
        _ => {
            return Err(MetadataArtifactError::invalid(
                crate::PACKAGE_METADATA_FILE,
                "invalid value projection step",
            ));
        }
    })
}
