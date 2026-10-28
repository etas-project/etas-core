use std::{collections::HashMap, sync::Arc};

use super::{identities::*, invalid, required, schema as p};
use crate::MetadataArtifactError as Error;
use etas_code::descriptor::*;

fn encode_reference(value: &AnnotationReferenceInfo) -> p::AnnotationReference {
    p::AnnotationReference {
        kind: Some(match value {
            AnnotationReferenceInfo::Declaration(key) => {
                p::annotation_reference::Kind::Declaration(encode_key(key))
            }
            AnnotationReferenceInfo::Module { package, path } => {
                p::annotation_reference::Kind::Module(p::ModuleReference {
                    package: package.fingerprint().as_bytes().to_vec(),
                    path: path.clone(),
                })
            }
        }),
    }
}
fn decode_reference(value: p::AnnotationReference) -> Result<AnnotationReferenceInfo, Error> {
    Ok(match required(value.kind, "annotation reference")? {
        p::annotation_reference::Kind::Declaration(key) => {
            AnnotationReferenceInfo::Declaration(decode_key(key)?)
        }
        p::annotation_reference::Kind::Module(value) => AnnotationReferenceInfo::Module {
            package: super::super::origins::package_key(value.package)?,
            path: value.path,
        },
    })
}

fn reference<'a>(
    value: &'a StaticValueInfo,
    ids: &mut HashMap<usize, u32>,
    pending: &mut Vec<&'a StaticValueInfo>,
) -> u32 {
    *ids.entry(value as *const StaticValueInfo as usize)
        .or_insert_with(|| {
            let id = pending.len() as u32;
            pending.push(value);
            id
        })
}

pub(super) fn encode(annotation: &AnnotationInfo) -> p::Annotation {
    let mut ids = HashMap::new();
    let mut pending = Vec::new();
    let arguments = annotation
        .arguments
        .iter()
        .map(|arg| p::StaticArgument {
            name: arg.name.clone(),
            value: reference(&arg.value, &mut ids, &mut pending),
        })
        .collect();
    let mut values = Vec::new();
    while values.len() < pending.len() {
        let value = pending[values.len()];
        use p::static_value::Kind as K;
        let kind = match value {
            StaticValueInfo::Literal(literal) => match literal {
                AnnotationLiteralInfo::Unit => K::Unit(true),
                AnnotationLiteralInfo::Bool(value) => K::Bool(*value),
                AnnotationLiteralInfo::Integer(value) => K::Integer(value.clone()),
                AnnotationLiteralInfo::Float(value) => K::Float(value.clone()),
                AnnotationLiteralInfo::String(value) => K::String(value.clone()),
                AnnotationLiteralInfo::Character(value) => K::Character(u32::from(*value)),
            },
            StaticValueInfo::Reference(value) => K::Reference(encode_reference(value)),
            StaticValueInfo::Sequence { kind, elements } => K::Sequence(p::StaticSequence {
                kind: match kind {
                    AnnotationSequenceKind::Array => 1,
                    AnnotationSequenceKind::List => 2,
                    AnnotationSequenceKind::Set => 3,
                    AnnotationSequenceKind::Tuple => 4,
                },
                elements: elements
                    .iter()
                    .map(|value| reference(value, &mut ids, &mut pending))
                    .collect(),
            }),
            StaticValueInfo::Record(fields) => K::Record(p::StaticRecord {
                fields: fields
                    .iter()
                    .map(|field| p::StaticField {
                        name: field.name.clone(),
                        value: reference(&field.value, &mut ids, &mut pending),
                    })
                    .collect(),
            }),
            StaticValueInfo::Constructor {
                kind,
                target,
                arguments,
            } => K::Constructor(p::StaticConstructor {
                kind: match kind {
                    AnnotationConstructorKind::Limit => 1,
                    AnnotationConstructorKind::Trace => 2,
                },
                target: Some(encode_reference(target)),
                arguments: arguments
                    .iter()
                    .map(|arg| p::StaticArgument {
                        name: arg.name.clone(),
                        value: reference(&arg.value, &mut ids, &mut pending),
                    })
                    .collect(),
            }),
        };
        values.push(p::StaticValue { kind: Some(kind) });
    }
    p::Annotation {
        path: annotation.path.clone(),
        arguments,
        values,
    }
}

pub(super) fn children(value: &p::StaticValue) -> Result<Vec<u32>, Error> {
    use p::static_value::Kind as K;
    Ok(
        match required(value.kind.as_ref(), "static annotation value")? {
            K::Sequence(value) => value.elements.clone(),
            K::Record(value) => value.fields.iter().map(|field| field.value).collect(),
            K::Constructor(value) => value.arguments.iter().map(|arg| arg.value).collect(),
            _ => Vec::new(),
        },
    )
}

pub(super) fn child_count(value: &p::StaticValue) -> Result<usize, Error> {
    use p::static_value::Kind as K;
    Ok(
        match required(value.kind.as_ref(), "static annotation value")? {
            K::Sequence(value) => value.elements.len(),
            K::Record(value) => value.fields.len(),
            K::Constructor(value) => value.arguments.len(),
            _ => 0,
        },
    )
}

/// Validate before creating recursive Arc values, including unreachable nodes.
pub(super) fn order(annotation: &p::Annotation) -> Result<Vec<usize>, Error> {
    let count = annotation.values.len();
    let raw = annotation.values.iter().try_fold(
        count.saturating_add(annotation.arguments.len()),
        |count, value| child_count(value).map(|children| count.saturating_add(children)),
    )?;
    if raw > super::super::MAX_METADATA_GRAPH_NODES {
        return Err(invalid("annotation graph exceeds the maximum node count"));
    }
    let edges = annotation
        .values
        .iter()
        .map(children)
        .collect::<Result<Vec<_>, _>>()?;
    if annotation
        .arguments
        .iter()
        .any(|arg| arg.value as usize >= count)
        || edges.iter().flatten().any(|child| *child as usize >= count)
    {
        return Err(invalid("annotation references a missing static value"));
    }
    let mut colors = vec![0u8; count];
    let mut heights = vec![0usize; count];
    let mut order = Vec::with_capacity(count);
    for root in 0..count {
        if colors[root] != 0 {
            continue;
        }
        colors[root] = 1;
        let mut stack = vec![(root, 0usize)];
        while let Some((node, position)) = stack.last_mut() {
            if let Some(child) = edges[*node].get(*position).copied() {
                *position += 1;
                let child = child as usize;
                match colors[child] {
                    0 => {
                        colors[child] = 1;
                        stack.push((child, 0));
                    }
                    1 => return Err(invalid("cyclic static annotation graph")),
                    _ => {}
                }
                if stack.len() > super::super::MAX_METADATA_GRAPH_DEPTH + 1 {
                    return Err(invalid("annotation graph exceeds the maximum depth"));
                }
            } else {
                let node = *node;
                heights[node] = edges[node]
                    .iter()
                    .map(|child| heights[*child as usize] + 1)
                    .max()
                    .unwrap_or(0);
                if heights[node] > super::super::MAX_METADATA_GRAPH_DEPTH {
                    return Err(invalid("annotation graph exceeds the maximum depth"));
                }
                colors[node] = 2;
                order.push(node);
                stack.pop();
            }
        }
    }
    let mut reachable = vec![false; count];
    let mut pending = annotation
        .arguments
        .iter()
        .map(|arg| arg.value as usize)
        .collect::<Vec<_>>();
    while let Some(index) = pending.pop() {
        if std::mem::replace(&mut reachable[index], true) {
            continue;
        }
        pending.extend(edges[index].iter().map(|child| *child as usize));
    }
    if reachable.contains(&false) {
        return Err(invalid("annotation graph contains unreachable values"));
    }
    Ok(order)
}

pub(super) fn decode(annotation: p::Annotation) -> Result<AnnotationInfo, Error> {
    let order = order(&annotation)?;
    let mut values = vec![None; annotation.values.len()];
    for index in order {
        use p::static_value::Kind as K;
        let value = match required(
            annotation.values[index].kind.as_ref(),
            "static annotation value",
        )? {
            K::Unit(true) => StaticValueInfo::Literal(AnnotationLiteralInfo::Unit),
            K::Unit(false) => return Err(invalid("invalid annotation unit marker")),
            K::Bool(value) => StaticValueInfo::Literal(AnnotationLiteralInfo::Bool(*value)),
            K::Integer(value) => {
                StaticValueInfo::Literal(AnnotationLiteralInfo::Integer(value.clone()))
            }
            K::Float(value) => {
                StaticValueInfo::Literal(AnnotationLiteralInfo::Float(value.clone()))
            }
            K::String(value) => {
                StaticValueInfo::Literal(AnnotationLiteralInfo::String(value.clone()))
            }
            K::Character(value) => StaticValueInfo::Literal(AnnotationLiteralInfo::Character(
                char::from_u32(*value).ok_or_else(|| invalid("invalid annotation character"))?,
            )),
            K::Reference(value) => StaticValueInfo::Reference(decode_reference(value.clone())?),
            K::Sequence(value) => StaticValueInfo::Sequence {
                kind: match value.kind {
                    1 => AnnotationSequenceKind::Array,
                    2 => AnnotationSequenceKind::List,
                    3 => AnnotationSequenceKind::Set,
                    4 => AnnotationSequenceKind::Tuple,
                    _ => return Err(invalid("unknown annotation sequence kind")),
                },
                elements: value
                    .elements
                    .iter()
                    .map(|id| get(&values, *id))
                    .collect::<Result<_, _>>()?,
            },
            K::Record(value) => StaticValueInfo::Record(
                value
                    .fields
                    .iter()
                    .map(|field| {
                        Ok(AnnotationFieldInfo {
                            name: field.name.clone(),
                            value: get(&values, field.value)?,
                        })
                    })
                    .collect::<Result<_, Error>>()?,
            ),
            K::Constructor(value) => StaticValueInfo::Constructor {
                kind: match value.kind {
                    1 => AnnotationConstructorKind::Limit,
                    2 => AnnotationConstructorKind::Trace,
                    _ => return Err(invalid("unknown annotation constructor kind")),
                },
                target: decode_reference(required(
                    value.target.clone(),
                    "annotation constructor target",
                )?)?,
                arguments: arguments(&value.arguments, &values)?,
            },
        };
        values[index] = Some(Arc::new(value));
    }
    Ok(AnnotationInfo {
        path: annotation.path,
        arguments: arguments(&annotation.arguments, &values)?,
    })
}

fn get(values: &[Option<Arc<StaticValueInfo>>], id: u32) -> Result<Arc<StaticValueInfo>, Error> {
    values
        .get(id as usize)
        .and_then(Option::as_ref)
        .cloned()
        .ok_or_else(|| invalid("static annotation dependency was not decoded"))
}
fn arguments(
    arguments: &[p::StaticArgument],
    values: &[Option<Arc<StaticValueInfo>>],
) -> Result<Vec<AnnotationArgumentInfo>, Error> {
    arguments
        .iter()
        .map(|arg| {
            Ok(AnnotationArgumentInfo {
                name: arg.name.clone(),
                value: get(values, arg.value)?,
            })
        })
        .collect()
}
