use super::{MetadataArtifactError, effects, invalid, schema};
use etas_code::descriptor::{GenericOwner, ParameterDefaultInfo, ParameterDefaultOwner};

pub(super) fn validate_wire(
    default: &schema::ParameterDefault,
    budget: &mut super::super::budget::MetadataGraphBudget,
) -> Result<(), MetadataArtifactError> {
    super::validate_generic_owner(
        default
            .owner
            .as_ref()
            .ok_or_else(|| invalid("default parameter has no owner"))?,
        budget,
    )?;
    budget.charge(
        default.revision.len() + default.spec_method.as_ref().map_or(0, String::len),
        "default parameter reference",
    )
}

pub(super) fn validate_model(
    default: &ParameterDefaultInfo,
    budget: &mut super::super::budget::MetadataGraphBudget,
) -> Result<(), MetadataArtifactError> {
    let value = encode(default);
    validate_wire(&value, budget)
}

pub(super) fn encode(default: &ParameterDefaultInfo) -> schema::ParameterDefault {
    let (owner, spec_method) = match &default.owner {
        ParameterDefaultOwner::Declaration(key) => (GenericOwner::Declaration(key.clone()), None),
        ParameterDefaultOwner::Method(method) => (GenericOwner::Method(method.clone()), None),
        ParameterDefaultOwner::SpecMethod { spec, name } => {
            (GenericOwner::Declaration(spec.clone()), Some(name.clone()))
        }
        ParameterDefaultOwner::Action { effect, name } => (
            GenericOwner::Action {
                effect: effect.clone(),
                name: name.clone(),
            },
            None,
        ),
    };
    schema::ParameterDefault {
        injected: default.injected,
        owner: Some(effects::encode_owner(&owner)),
        spec_method,
        parameter: default.parameter,
        revision: default.revision.as_bytes().to_vec(),
    }
}

pub(super) fn decode(
    default: schema::ParameterDefault,
) -> Result<ParameterDefaultInfo, MetadataArtifactError> {
    let owner = effects::decode_owner(
        default
            .owner
            .ok_or_else(|| invalid("default parameter has no canonical owner"))?,
    )?;
    let owner = match (owner, default.spec_method) {
        (GenericOwner::Declaration(spec), Some(name)) => {
            ParameterDefaultOwner::SpecMethod { spec, name }
        }
        (GenericOwner::Declaration(key), None) => ParameterDefaultOwner::Declaration(key),
        (GenericOwner::Method(method), None) => ParameterDefaultOwner::Method(method),
        (GenericOwner::Action { effect, name }, None) => {
            ParameterDefaultOwner::Action { effect, name }
        }
        _ => return Err(invalid("invalid default parameter owner")),
    };
    owner
        .validate()
        .map_err(|_| invalid("invalid default parameter declaration kind"))?;
    let revision: [u8; 32] = default
        .revision
        .try_into()
        .map_err(|_| invalid("invalid default parameter revision fingerprint"))?;
    Ok(ParameterDefaultInfo {
        injected: default.injected,
        owner,
        parameter: default.parameter,
        revision: etas_code::CodeFingerprint::from_bytes(revision),
    })
}
