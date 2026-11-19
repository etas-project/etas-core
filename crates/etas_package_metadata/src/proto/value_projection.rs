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
    };
    P {
        kind,
        name,
        position,
    }
}

pub(super) fn decode(step: P) -> Result<S, MetadataArtifactError> {
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
        _ => {
            return Err(MetadataArtifactError::invalid(
                crate::PACKAGE_METADATA_FILE,
                "invalid value projection step",
            ));
        }
    })
}
