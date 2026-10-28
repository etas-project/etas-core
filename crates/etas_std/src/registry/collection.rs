use crate::{
    FlowDecl, FlowSourceMethodKind, IntrinsicDescriptor, IntrinsicDispatch, IntrinsicPurity,
    LoweringHint, StdCollectionKind as K, StdCollectionOperation as O, StdRegistryValidationError,
    StdSpecRef, StdType,
};

pub(super) fn validate(
    flow: &FlowDecl,
    descriptor: &IntrinsicDescriptor,
) -> Result<(), StdRegistryValidationError> {
    let LoweringHint::Collection(operation) = descriptor.lowering else {
        return Ok(());
    };
    let valid = operation.is_supported()
        && descriptor.id == operation.id()
        && descriptor.dispatch == IntrinsicDispatch::Runtime
        && descriptor.purity == IntrinsicPurity::Pure
        && signature_matches(flow, operation.kind, operation.operation);
    if valid {
        Ok(())
    } else {
        Err(StdRegistryValidationError { symbol: descriptor.qualified_path.join("."),
            reason: "collection descriptor requires a stable supported operation and its exact checked signature".into() })
    }
}

fn index_param(flow: &FlowDecl, ty: &StdType) -> bool {
    matches!(ty, StdType::Var(name) if flow.type_params.iter().any(|param|
        param.name == *name && param.bounds.contains(&StdSpecRef::new(&["std", "core", "Index"]))))
}

fn signature_matches(flow: &FlowDecl, kind: K, operation: O) -> bool {
    let Some(method) = &flow.source_method else {
        return false;
    };
    let marker = if operation == O::At {
        crate::FlowSourceMethodOperation::CheckedIndex
    } else {
        crate::FlowSourceMethodOperation::Call
    };
    if method.operation != marker {
        return false;
    }
    if method.name != (crate::StdCollectionIntrinsic { kind, operation }).method_name() {
        return false;
    }
    let receiver = &method.receiver;
    let (element, key, priority) = match (kind, receiver) {
        (K::Array, StdType::Array(t))
        | (K::List, StdType::List(t))
        | (K::Slice, StdType::Slice(t))
        | (K::Range, StdType::Range(t)) => (t.as_ref(), t.as_ref(), t.as_ref()),
        (K::Map, StdType::Map { key, value }) => (value.as_ref(), key.as_ref(), key.as_ref()),
        (kind, StdType::NamedApplied { name, args }) => {
            let expected = match kind {
                K::Deque => "Deque",
                K::Queue => "Queue",
                K::Stack => "Stack",
                K::PriorityQueue => "PriorityQueue",
                K::OrderedMap => "OrderedMap",
                K::OrderedSet => "OrderedSet",
                _ => return false,
            };
            if name != expected {
                return false;
            }
            match (kind, args.as_slice()) {
                (K::OrderedMap, [k, v]) => (v, k, k),
                (K::PriorityQueue, [t, p]) => (t, t, p),
                (K::Deque | K::Queue | K::Stack | K::OrderedSet, [t]) => (t, t, t),
                _ => return false,
            }
        }
        _ => return false,
    };
    let type_member = matches!(operation, O::New | O::WithCompare | O::Closed | O::Open);
    if method.kind
        != if type_member {
            FlowSourceMethodKind::TypeMember
        } else {
            FlowSourceMethodKind::ValueMethod
        }
    {
        return false;
    }
    let r = receiver.clone();
    let t = element.clone();
    let (input, output) = match operation {
        O::Get | O::At => {
            let Some(index) = flow.params.get(1) else {
                return false;
            };
            if matches!(kind, K::Map | K::OrderedMap) {
                if index != key {
                    return false;
                }
            } else if !index_param(flow, index) {
                return false;
            }
            (
                vec![r, index.clone()],
                if operation == O::At {
                    t
                } else {
                    StdType::Option(Box::new(t))
                },
            )
        }
        O::Head => (vec![r], StdType::Option(Box::new(t))),
        O::Tail => (vec![r.clone()], StdType::Option(Box::new(r))),
        O::Push | O::PushFront | O::PushBack => {
            let mut input = vec![r.clone(), t];
            if kind == K::PriorityQueue {
                input.push(priority.clone());
            }
            (input, r)
        }
        O::Pop | O::PopFront | O::PopBack => (
            vec![r.clone()],
            StdType::Tuple(vec![r, StdType::Option(Box::new(t))]),
        ),
        O::Extend => (vec![r.clone(), r.clone()], r),
        O::ToArray => (vec![r], StdType::Array(Box::new(t))),
        O::New => (vec![], r),
        O::WithCompare => {
            let key = if kind == K::PriorityQueue {
                priority
            } else {
                key
            };
            (
                vec![StdType::Function {
                    input: vec![key.clone(), key.clone()],
                    output: Box::new(StdType::parse("i32")),
                    effects: vec![],
                }],
                r,
            )
        }
        O::Insert => {
            let mut input = vec![r.clone(), key.clone()];
            if kind == K::OrderedMap {
                input.push(t);
            }
            (input, r)
        }
        O::Contains | O::ContainsKey => (vec![r, key.clone()], StdType::parse("bool")),
        O::Closed | O::Open => {
            if !index_param(flow, element) {
                return false;
            }
            (vec![t.clone(), t], r)
        }
    };
    flow.params == input
        && flow.output == output
        && (operation == O::At || flow.public_effects.is_empty())
        && flow.requested_actions.is_empty()
}
