use crate::{GenericParamKind, SpecMethod, TypeKind};

pub(crate) fn validate(method: &SpecMethod) -> Result<(), String> {
    let Some(index) = method.receiver_param else {
        return Ok(());
    };
    let signature = method
        .signature
        .as_ref()
        .ok_or("spec receiver requires a callable signature")?;
    let parameter = signature
        .generic_params
        .get(index as usize)
        .ok_or("spec receiver parameter index is out of bounds")?;
    if parameter.kind != GenericParamKind::Type {
        return Err("spec receiver parameter must have type kind".into());
    }
    if signature.input.first().is_none_or(|ty| {
        ty.kind != TypeKind::Var || ty.name != parameter.name || !ty.children.is_empty()
    }) {
        return Err("spec receiver must bind the first callable input type".into());
    }
    Ok(())
}
