use super::{
    CallableOutcomeInfo, DescriptorError, DescriptorGraphInput, DescriptorNode, EffectRowInfoRef,
    GenericParameterRef, StaticEffectArgInfo, TraceInfo, TraceInfoRef, TypeInfo, TypeInfoRef,
    VariantPayloadInfo,
};

pub(super) fn node_index(input: &DescriptorGraphInput, node: DescriptorNode) -> Option<usize> {
    match node {
        DescriptorNode::Type(reference) => {
            ((reference.0 as usize) < input.types.len()).then_some(reference.0 as usize)
        }
        DescriptorNode::Row(reference) => ((reference.0 as usize) < input.rows.len())
            .then_some(input.types.len() + reference.0 as usize),
        DescriptorNode::Binder(reference) => ((reference.0 as usize) < input.binders.len())
            .then_some(input.types.len() + input.rows.len() + reference.0 as usize),
        DescriptorNode::Trace(reference) => ((reference.0 as usize) < input.traces.len())
            .then_some(
                input.types.len() + input.rows.len() + input.binders.len() + reference.0 as usize,
            ),
    }
}

pub(super) fn node_at(input: &DescriptorGraphInput, index: usize) -> DescriptorNode {
    if index < input.types.len() {
        DescriptorNode::Type(TypeInfoRef(index as u32))
    } else if index < input.types.len() + input.rows.len() {
        DescriptorNode::Row(EffectRowInfoRef((index - input.types.len()) as u32))
    } else if index < input.types.len() + input.rows.len() + input.binders.len() {
        DescriptorNode::Binder(GenericParameterRef(
            (index - input.types.len() - input.rows.len()) as u32,
        ))
    } else {
        DescriptorNode::Trace(TraceInfoRef(
            (index - input.types.len() - input.rows.len() - input.binders.len()) as u32,
        ))
    }
}

pub(super) fn references(
    input: &DescriptorGraphInput,
    node: DescriptorNode,
    expand_definitions: bool,
    mut visit: impl FnMut(DescriptorNode),
) {
    match node {
        DescriptorNode::Type(reference) => match &input.types[reference.0 as usize] {
            TypeInfo::Primitive(_) => {}
            TypeInfo::Builtin { arguments, .. } => {
                for argument in arguments {
                    visit(DescriptorNode::Type(*argument));
                }
            }
            TypeInfo::Parameter(parameter) => visit(DescriptorNode::Binder(*parameter)),
            TypeInfo::Tuple(elements) => {
                for element in elements {
                    visit(DescriptorNode::Type(*element));
                }
            }
            TypeInfo::Record(fields) => {
                for field in fields {
                    visit(DescriptorNode::Type(field.ty));
                }
            }
            TypeInfo::Nominal {
                parameters,
                representation,
                ..
            } => {
                for parameter in parameters {
                    visit(DescriptorNode::Binder(*parameter));
                }
                if expand_definitions && let Some(representation) = representation {
                    visit(DescriptorNode::Type(*representation));
                }
            }
            TypeInfo::Alias {
                parameters, target, ..
            } => {
                for parameter in parameters {
                    visit(DescriptorNode::Binder(*parameter));
                }
                visit(DescriptorNode::Type(*target));
            }
            TypeInfo::Enum {
                parameters,
                variants,
                ..
            } => {
                for parameter in parameters {
                    visit(DescriptorNode::Binder(*parameter));
                }
                if expand_definitions {
                    for variant in variants {
                        match &variant.payload {
                            VariantPayloadInfo::None => {}
                            VariantPayloadInfo::Positional(types) => {
                                for ty in types {
                                    visit(DescriptorNode::Type(*ty));
                                }
                            }
                            VariantPayloadInfo::Named(fields) => {
                                for field in fields {
                                    visit(DescriptorNode::Type(field.ty));
                                }
                            }
                        }
                    }
                }
            }
            TypeInfo::Applied {
                constructor,
                arguments,
            } => {
                visit(DescriptorNode::Type(*constructor));
                for argument in arguments {
                    visit(DescriptorNode::Type(*argument));
                }
            }
            TypeInfo::Function(signature) => {
                for generic in &signature.generics {
                    visit(DescriptorNode::Binder(*generic));
                }
                for parameter in &signature.parameters {
                    visit(DescriptorNode::Type(parameter.ty));
                }
                visit(DescriptorNode::Type(signature.output));
                if let Some(ty) = signature.declared_output {
                    visit(DescriptorNode::Type(ty));
                }
                if let Some(row) = signature.effects {
                    visit(DescriptorNode::Row(row));
                }
            }
            TypeInfo::Handler {
                handled,
                produced,
                result,
            } => {
                visit(DescriptorNode::Row(*handled));
                if let Some(produced) = produced {
                    visit(DescriptorNode::Row(*produced));
                }
                if let Some(result) = result {
                    visit(DescriptorNode::Type(*result));
                }
            }
        },
        DescriptorNode::Row(reference) => {
            let row = &input.rows[reference.0 as usize];
            if let Some(tail) = row.tail {
                visit(DescriptorNode::Binder(tail));
            }
            for effect in &row.effects {
                for arg in &effect.arguments {
                    if let StaticEffectArgInfo::Type(ty) = arg {
                        visit(DescriptorNode::Type(*ty));
                    }
                }
            }
        }
        DescriptorNode::Binder(reference) => {
            if expand_definitions {
                for bound in &input.binders[reference.0 as usize].bounds {
                    for argument in &bound.arguments {
                        visit(DescriptorNode::Type(*argument));
                    }
                }
            }
        }
        DescriptorNode::Trace(reference) => match &input.traces[reference.0 as usize] {
            TraceInfo::Empty => {}
            TraceInfo::Outcome(outcome) => match outcome {
                CallableOutcomeInfo::Returned { ty, .. } => visit(DescriptorNode::Type(*ty)),
                CallableOutcomeInfo::Raised { error } => visit(DescriptorNode::Type(*error)),
                CallableOutcomeInfo::NonReturn => {}
                CallableOutcomeInfo::ParameterNonReturn(call) => {
                    visit(DescriptorNode::Type(call.signature))
                }
            },
            TraceInfo::ParameterCall(call) => visit(DescriptorNode::Type(call.signature)),
            TraceInfo::Event(effect) => {
                for argument in &effect.arguments {
                    if let StaticEffectArgInfo::Type(ty) = argument {
                        visit(DescriptorNode::Type(*ty));
                    }
                }
            }
            TraceInfo::Seq(parts) | TraceInfo::Choice(parts) => {
                for part in parts {
                    visit(DescriptorNode::Trace(*part));
                }
            }
            TraceInfo::Repeat(inner) => visit(DescriptorNode::Trace(*inner)),
            TraceInfo::UnknownOrder(actions) | TraceInfo::Widened { actions, .. } => {
                visit(DescriptorNode::Row(*actions));
                if let TraceInfo::Widened {
                    parameter_calls, ..
                } = &input.traces[reference.0 as usize]
                {
                    for call in parameter_calls {
                        visit(DescriptorNode::Type(call.signature));
                    }
                }
            }
        },
    }
}

pub(super) fn validate_expansion(
    input: &DescriptorGraphInput,
    adjacency: &[Vec<usize>],
    limit: usize,
) -> Result<(), DescriptorError> {
    let mut colors = vec![0u8; adjacency.len()];
    let mut heights = vec![0usize; adjacency.len()];
    for root in 0..adjacency.len() {
        if colors[root] != 0 {
            continue;
        }
        colors[root] = 1;
        let mut stack = vec![(root, 0usize)];
        while let Some((node, cursor)) = stack.last_mut() {
            if let Some(&child) = adjacency[*node].get(*cursor) {
                *cursor += 1;
                match colors[child] {
                    0 => {
                        colors[child] = 1;
                        stack.push((child, 0));
                    }
                    1 => {
                        return Err(DescriptorError::UnguardedCycle {
                            node: node_at(input, child),
                        });
                    }
                    _ => {}
                }
            } else {
                let height = adjacency[*node]
                    .iter()
                    .map(|child| heights[*child])
                    .max()
                    .unwrap_or(0)
                    .saturating_add(1);
                if height > limit {
                    return Err(DescriptorError::Limit {
                        resource: "expansion depth",
                        limit,
                    });
                }
                heights[*node] = height;
                colors[*node] = 2;
                stack.pop();
            }
        }
    }
    Ok(())
}
