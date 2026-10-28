use etas_code::descriptor::*;

use super::{identities::*, invalid, required, schema as p};
use crate::MetadataArtifactError as Error;

pub(super) fn encode_effect(effect: &EffectInfo) -> p::Effect {
    p::Effect {
        target: Some(encode_target(&effect.target)),
        arguments: effect
            .arguments
            .iter()
            .map(|argument| p::StaticArg {
                kind: Some(match argument {
                    StaticEffectArgInfo::Type(ty) => p::static_arg::Kind::Type(ty.0),
                    StaticEffectArgInfo::Resource(key) => {
                        p::static_arg::Kind::Resource(encode_key(key))
                    }
                    StaticEffectArgInfo::String(value) => {
                        p::static_arg::Kind::String(value.clone())
                    }
                    StaticEffectArgInfo::Integer(value) => {
                        p::static_arg::Kind::Integer(value.clone())
                    }
                    StaticEffectArgInfo::Wildcard => p::static_arg::Kind::Wildcard(true),
                    StaticEffectArgInfo::Callable(id) => {
                        p::static_arg::Kind::Callable(encode_callable(id))
                    }
                }),
            })
            .collect(),
    }
}

pub(super) fn encode_target(target: &EffectTargetInfo) -> p::EffectTarget {
    let (key, action) = match target {
        EffectTargetInfo::Effect(key) => (key, None),
        EffectTargetInfo::Action { effect, name } => (effect, Some(name.clone())),
    };
    p::EffectTarget {
        effect: Some(encode_key(key)),
        action,
    }
}

pub(super) fn decode_effect(effect: p::Effect) -> Result<EffectInfo, Error> {
    let target = required(effect.target, "effect target")?;
    let target = decode_target(target)?;
    Ok(EffectInfo {
        target,
        arguments: effect
            .arguments
            .into_iter()
            .map(|argument| {
                Ok(match required(argument.kind, "static effect argument")? {
                    p::static_arg::Kind::Type(ty) => StaticEffectArgInfo::Type(TypeInfoRef(ty)),
                    p::static_arg::Kind::Resource(key) => {
                        StaticEffectArgInfo::Resource(decode_key(key)?)
                    }
                    p::static_arg::Kind::String(value) => StaticEffectArgInfo::String(value),
                    p::static_arg::Kind::Integer(value) => StaticEffectArgInfo::Integer(value),
                    p::static_arg::Kind::Wildcard(true) => StaticEffectArgInfo::Wildcard,
                    p::static_arg::Kind::Wildcard(false) => {
                        return Err(invalid("invalid wildcard descriptor marker"));
                    }
                    p::static_arg::Kind::Callable(id) => {
                        StaticEffectArgInfo::Callable(decode_callable(id)?)
                    }
                })
            })
            .collect::<Result<_, Error>>()?,
    })
}

pub(super) fn decode_target(target: p::EffectTarget) -> Result<EffectTargetInfo, Error> {
    let key = decode_key(required(target.effect, "effect identity")?)?;
    Ok(match target.action {
        Some(name) => EffectTargetInfo::Action { effect: key, name },
        None => EffectTargetInfo::Effect(key),
    })
}

pub(super) fn encode_spec(spec: &SpecInfo) -> p::Spec {
    p::Spec {
        definition: Some(encode_key(&spec.definition)),
        arguments: spec.arguments.iter().map(|ty| ty.0).collect(),
    }
}
pub(super) fn decode_spec(spec: p::Spec) -> Result<SpecInfo, Error> {
    Ok(SpecInfo {
        definition: decode_key(required(spec.definition, "spec identity")?)?,
        arguments: spec.arguments.into_iter().map(TypeInfoRef).collect(),
    })
}

pub(super) fn encode_row(row: &EffectRowInfo) -> p::Row {
    p::Row {
        effects: row.effects.iter().map(encode_effect).collect(),
        tail: row.tail.map(|tail| tail.0),
    }
}
pub(super) fn decode_row(row: p::Row) -> Result<EffectRowInfo, Error> {
    Ok(EffectRowInfo {
        effects: row
            .effects
            .into_iter()
            .map(decode_effect)
            .collect::<Result<_, _>>()?,
        tail: row.tail.map(GenericParameterRef),
    })
}

pub(super) fn encode_binder(binder: &GenericParameterInfo) -> p::Binder {
    let owner = match &binder.owner {
        GenericOwner::Declaration(key) => p::callable::Kind::Declaration(encode_key(key)),
        GenericOwner::Method(method) => p::callable::Kind::Method(encode_method(method)),
    };
    p::Binder {
        owner: Some(p::Callable { kind: Some(owner) }),
        position: binder.position,
        name: binder.name.clone(),
        kind: match binder.kind {
            GenericParameterKind::Type => 1,
            GenericParameterKind::EffectRow => 2,
        },
        bounds: binder
            .bounds
            .iter()
            .map(|bound| p::Spec {
                definition: Some(encode_key(&bound.definition)),
                arguments: bound.arguments.iter().map(|ty| ty.0).collect(),
            })
            .collect(),
    }
}

pub(super) fn decode_binder(binder: p::Binder) -> Result<GenericParameterInfo, Error> {
    let owner = match required(
        required(binder.owner, "generic owner")?.kind,
        "generic owner kind",
    )? {
        p::callable::Kind::Declaration(key) => GenericOwner::Declaration(decode_key(key)?),
        p::callable::Kind::Method(method) => GenericOwner::Method(decode_method(method)?),
    };
    Ok(GenericParameterInfo {
        owner,
        position: binder.position,
        name: binder.name,
        kind: match binder.kind {
            1 => GenericParameterKind::Type,
            2 => GenericParameterKind::EffectRow,
            _ => return Err(invalid("unknown generic descriptor kind")),
        },
        bounds: binder
            .bounds
            .into_iter()
            .map(|bound| {
                Ok(SpecInfo {
                    definition: decode_key(required(bound.definition, "spec bound identity")?)?,
                    arguments: bound.arguments.into_iter().map(TypeInfoRef).collect(),
                })
            })
            .collect::<Result<_, Error>>()?,
    })
}
