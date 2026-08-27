use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdDecl,
    StdEffectRef, StdGenericParam, StdIntrinsicId, StdRegistryBuilder, StdSpecRef, StdSymbolKind,
    StdType, TypeDecl, TypeDeclKind, intrinsic,
};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(
        &["std", "collections"],
        "Standard collection type declarations.",
    );
    for (name, params) in [
        ("Array", &["T"][..]),
        ("List", &["T"][..]),
        ("Map", &["K", "V"][..]),
        ("Set", &["T"][..]),
        ("Range", &["I"][..]),
        ("Slice", &["T"][..]),
        ("Deque", &["T"][..]),
        ("Queue", &["T"][..]),
        ("Stack", &["T"][..]),
        ("PriorityQueue", &["T", "P"][..]),
        ("OrderedMap", &["K", "V"][..]),
        ("OrderedSet", &["T"][..]),
    ] {
        let symbol = builder.symbol(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(TypeDecl::generic(name, params, TypeDeclKind::Support)),
            "Standard generic collection type.",
        );
        builder.prelude(name, symbol);
    }
    for (name, summary) in [
        (
            "LengthInput",
            "Compiler-known support constraint accepted by std.collections.len.",
        ),
        (
            "EmptinessInput",
            "Compiler-known support constraint accepted by std.collections.is_empty.",
        ),
    ] {
        builder.symbol(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(TypeDecl::generic(name, &[], TypeDeclKind::Support)),
            summary,
        );
    }
    builder.symbol_with_intrinsic(
        module,
        "len",
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::pure("len", &["LengthInput"], "usize")),
        "Return the length of an Array, List, Slice, Map, or string value.",
        Some(IntrinsicDescriptor {
            id: StdIntrinsicId(intrinsic::pure::LIST_LEN),
            qualified_path: vec!["std".into(), "collections".into(), "len".into()],
            purity: IntrinsicPurity::Pure,
            dispatch: IntrinsicDispatch::PureKernel,
            lowering: LoweringHint::PureBuiltin,
            latent_effect: crate::IntrinsicLatentEffect::None,
            memory_access: crate::IntrinsicMemoryAccess::None,
            runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
        }),
    );
    builder.symbol_with_intrinsic(
        module,
        "is_empty",
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::pure("is_empty", &["EmptinessInput"], "bool")),
        "Return whether an Array, List, Slice, Map, or string value is empty.",
        Some(IntrinsicDescriptor {
            id: StdIntrinsicId(intrinsic::pure::LIST_IS_EMPTY),
            qualified_path: vec!["std".into(), "collections".into(), "is_empty".into()],
            purity: IntrinsicPurity::Pure,
            dispatch: IntrinsicDispatch::PureKernel,
            lowering: LoweringHint::PureBuiltin,
            latent_effect: crate::IntrinsicLatentEffect::None,
            memory_access: crate::IntrinsicMemoryAccess::None,
            runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
        }),
    );
    builder.symbol_with_intrinsic(
        module,
        "contains_key",
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::with_type_params_actions(
            "contains_key",
            &[StdGenericParam::new("K"), StdGenericParam::new("V")],
            &["Map[K, V]", "K"],
            "bool",
            &[],
            &[],
        )),
        "Return whether a map contains the requested key.",
        Some(IntrinsicDescriptor {
            id: StdIntrinsicId(intrinsic::pure::MAP_CONTAINS_KEY),
            qualified_path: vec!["std".into(), "collections".into(), "contains_key".into()],
            purity: IntrinsicPurity::Pure,
            dispatch: IntrinsicDispatch::PureKernel,
            lowering: LoweringHint::PureBuiltin,
            latent_effect: crate::IntrinsicLatentEffect::None,
            memory_access: crate::IntrinsicMemoryAccess::None,
            runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
        }),
    );

    for (name, params, output, error_type, docs) in [
        (
            "get",
            &["Array[T]", "Index"][..],
            "Option[T]",
            None,
            "Return an array element when the checked index is in range.",
        ),
        (
            "get",
            &["List[T]", "Index"][..],
            "Option[T]",
            None,
            "Return a list element when the checked index is in range.",
        ),
        (
            "at",
            &["Array[T]", "Index"][..],
            "T",
            Some("IndexError"),
            "Return an array element or raise Error[IndexError] when out of range.",
        ),
        (
            "at",
            &["List[T]", "Index"][..],
            "T",
            Some("IndexError"),
            "Return a list element or raise Error[IndexError] when out of range.",
        ),
        (
            "push",
            &["Array[T]", "T"][..],
            "Array[T]",
            None,
            "Return a new array with the value appended.",
        ),
        (
            "push",
            &["List[T]", "T"][..],
            "List[T]",
            None,
            "Return a new list with the value prepended.",
        ),
        (
            "pop",
            &["Array[T]"][..],
            "(Array[T], Option[T])",
            None,
            "Return a new array without the last value and the removed value.",
        ),
        (
            "pop",
            &["List[T]"][..],
            "(List[T], Option[T])",
            None,
            "Return a new list without the head value and the removed value.",
        ),
        (
            "extend",
            &["Array[T]", "Array[T]"][..],
            "Array[T]",
            None,
            "Return a new array with another array appended.",
        ),
        (
            "extend",
            &["List[T]", "List[T]"][..],
            "List[T]",
            None,
            "Return a new list with another list prepended in order.",
        ),
        (
            "slice_get",
            &["Slice[T]", "Index"][..],
            "Option[T]",
            None,
            "Return a slice element when the checked index is in range.",
        ),
        (
            "slice_at",
            &["Slice[T]", "Index"][..],
            "T",
            Some("IndexError"),
            "Return a slice element or raise Error[IndexError] when out of range.",
        ),
        (
            "to_array",
            &["Slice[T]"][..],
            "Array[T]",
            None,
            "Materialize a slice as an array.",
        ),
        (
            "map_get",
            &["Map[K, V]", "K"][..],
            "Option[V]",
            None,
            "Return a map value when the key exists.",
        ),
    ] {
        let receiver = params
            .first()
            .copied()
            .expect("collection operation must declare a receiver");
        let receiver_name = receiver.split('[').next().unwrap_or(receiver);
        let source_method = match name {
            "slice_get" | "map_get" => "get",
            "slice_at" => "at",
            _ => name,
        };
        let symbol_name = match receiver_name {
            "Array" => format!("array_{name}"),
            "List" => format!("list_{name}"),
            _ => name.to_owned(),
        };
        let type_params = match receiver_name {
            "Map" => vec![StdGenericParam::new("K"), StdGenericParam::new("V")],
            _ => vec![StdGenericParam::new("T")],
        };
        let effects = error_type
            .map(|error_type| vec![StdEffectRef::typed(&["Error"], StdType::parse(error_type))])
            .unwrap_or_default();
        let decl =
            FlowDecl::with_type_params_actions(name, &type_params, params, output, &effects, &[])
                .with_value_method(receiver, source_method);
        builder.symbol(
            module,
            &symbol_name,
            StdSymbolKind::Flow,
            StdDecl::Flow(decl),
            docs,
        );
    }
    for (name, docs) in [
        ("closed", "Construct a closed range."),
        ("open", "Construct an open range."),
    ] {
        let decl = FlowDecl::with_type_params_actions(
            name,
            &[StdGenericParam::bounded(
                "T",
                &[StdSpecRef::new(&["std", "core", "Index"])],
            )],
            &["T", "T"],
            "Range[T]",
            &[],
            &[],
        )
        .with_type_member_method("Range[T]", name);
        builder.symbol(module, name, StdSymbolKind::Flow, StdDecl::Flow(decl), docs);
    }
    for (name, params, output, receiver, method, type_member, docs) in [
        (
            "deque_new",
            &[][..],
            "Deque[T]",
            "Deque[T]",
            "new",
            true,
            "Construct an empty deque with explicit type arguments.",
        ),
        (
            "deque_push_front",
            &["Deque[T]", "T"][..],
            "Deque[T]",
            "Deque[T]",
            "push_front",
            false,
            "Return a deque with a value inserted at the front.",
        ),
        (
            "deque_push_back",
            &["Deque[T]", "T"][..],
            "Deque[T]",
            "Deque[T]",
            "push_back",
            false,
            "Return a deque with a value appended at the back.",
        ),
        (
            "deque_pop_front",
            &["Deque[T]"][..],
            "(Deque[T], Option[T])",
            "Deque[T]",
            "pop_front",
            false,
            "Return a deque without the first value and the removed value.",
        ),
        (
            "deque_pop_back",
            &["Deque[T]"][..],
            "(Deque[T], Option[T])",
            "Deque[T]",
            "pop_back",
            false,
            "Return a deque without the last value and the removed value.",
        ),
        (
            "queue_new",
            &[][..],
            "Queue[T]",
            "Queue[T]",
            "new",
            true,
            "Construct an empty queue with explicit type arguments.",
        ),
        (
            "queue_push",
            &["Queue[T]", "T"][..],
            "Queue[T]",
            "Queue[T]",
            "push",
            false,
            "Return a queue with a value enqueued.",
        ),
        (
            "queue_pop",
            &["Queue[T]"][..],
            "(Queue[T], Option[T])",
            "Queue[T]",
            "pop",
            false,
            "Return a queue without the next value and the removed value.",
        ),
        (
            "stack_new",
            &[][..],
            "Stack[T]",
            "Stack[T]",
            "new",
            true,
            "Construct an empty stack with explicit type arguments.",
        ),
        (
            "stack_push",
            &["Stack[T]", "T"][..],
            "Stack[T]",
            "Stack[T]",
            "push",
            false,
            "Return a stack with a value pushed on top.",
        ),
        (
            "stack_pop",
            &["Stack[T]"][..],
            "(Stack[T], Option[T])",
            "Stack[T]",
            "pop",
            false,
            "Return a stack without the top value and the removed value.",
        ),
        (
            "priority_queue_new",
            &[][..],
            "PriorityQueue[T, P]",
            "PriorityQueue[T, P]",
            "new",
            true,
            "Construct an empty priority queue with explicit type arguments.",
        ),
        (
            "priority_queue_push",
            &["PriorityQueue[T, P]", "T", "P"][..],
            "PriorityQueue[T, P]",
            "PriorityQueue[T, P]",
            "push",
            false,
            "Return a priority queue with a prioritized value.",
        ),
        (
            "priority_queue_pop",
            &["PriorityQueue[T, P]"][..],
            "(PriorityQueue[T, P], Option[T])",
            "PriorityQueue[T, P]",
            "pop",
            false,
            "Return a priority queue without the next value and the removed value.",
        ),
        (
            "ordered_map_new",
            &[][..],
            "OrderedMap[K, V]",
            "OrderedMap[K, V]",
            "new",
            true,
            "Construct an empty ordered map with explicit type arguments.",
        ),
        (
            "ordered_map_get",
            &["OrderedMap[K, V]", "K"][..],
            "Option[V]",
            "OrderedMap[K, V]",
            "get",
            false,
            "Return an ordered-map value when the key exists.",
        ),
        (
            "ordered_map_insert",
            &["OrderedMap[K, V]", "K", "V"][..],
            "OrderedMap[K, V]",
            "OrderedMap[K, V]",
            "insert",
            false,
            "Return an ordered map with a key/value inserted or replaced.",
        ),
        (
            "ordered_map_contains_key",
            &["OrderedMap[K, V]", "K"][..],
            "bool",
            "OrderedMap[K, V]",
            "contains_key",
            false,
            "Return whether an ordered map contains the requested key.",
        ),
        (
            "ordered_set_new",
            &[][..],
            "OrderedSet[T]",
            "OrderedSet[T]",
            "new",
            true,
            "Construct an empty ordered set with explicit type arguments.",
        ),
        (
            "ordered_set_insert",
            &["OrderedSet[T]", "T"][..],
            "OrderedSet[T]",
            "OrderedSet[T]",
            "insert",
            false,
            "Return an ordered set containing the value.",
        ),
        (
            "ordered_set_contains",
            &["OrderedSet[T]", "T"][..],
            "bool",
            "OrderedSet[T]",
            "contains",
            false,
            "Return whether an ordered set contains the value.",
        ),
    ] {
        let type_params = if receiver.starts_with("PriorityQueue") {
            vec![StdGenericParam::new("T"), StdGenericParam::new("P")]
        } else if receiver.starts_with("OrderedMap") {
            vec![StdGenericParam::new("K"), StdGenericParam::new("V")]
        } else {
            vec![StdGenericParam::new("T")]
        };
        let decl = FlowDecl::with_type_params_actions(name, &type_params, params, output, &[], &[]);
        let decl = if type_member {
            decl.with_type_member_method(receiver, method)
        } else {
            decl.with_value_method(receiver, method)
        };
        builder.symbol(module, name, StdSymbolKind::Flow, StdDecl::Flow(decl), docs);
    }
}
