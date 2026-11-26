module std.compiler;

public import std.agent.ToolRef;

// These are descriptive values. Possessing an identity or descriptor does not
// grant access to a retained definition, execution context or compiler session.
public enum DefinitionKind {
    Type,
    Alias,
    Enum,
    Spec,
    Effect,
    Flow,
    Agent,
    Tool,
    Resource,
    Value,
    EnumVariant,
    Requirement,
    Protocol,
}

public type DefinitionId = {
    package: string,
    module: Array<string>,
    declaration: Array<string>,
    kind: DefinitionKind,
};

public type AgentId = { definition: DefinitionId };

public type ImplementationId = {
    package: string,
    module: Array<string>,
    subject: DefinitionId,
    selected_spec: Option<DefinitionId>,
    header: string,
};

public enum CallableId {
    Declaration(DefinitionId),
    Method {
        owner: DefinitionId,
        implementation: Option<ImplementationId>,
        name: string,
        overload: Option<string>,
    },
}

public enum GenericOwnerInfo {
    Declaration(DefinitionId),
    Method(CallableId),
    Implementation(ImplementationId),
    ImplementationPattern(DefinitionId),
    Action { effect: DefinitionId, name: string },
    ActionOverload { effect: DefinitionId, name: string, signature: string },
}

public enum ParameterDefaultOwner {
    Declaration(DefinitionId),
    Method(CallableId),
    SpecMethod { spec: DefinitionId, name: string },
    Action { effect: DefinitionId, name: string },
}

public type ParameterDefaultInfo = {
    injected: bool,
    owner: ParameterDefaultOwner,
    parameter: usize,
    revision: string,
};

public type GenericParameterRef = {
    owner: GenericOwnerInfo,
    position: usize,
};

// A reference ends expansion of a recursive named type. Structural members
// recurse normally; descriptors do not need cyclic heap objects or native IDs.
public enum TypeInfo {
    Primitive(string),
    Builtin { name: string, arguments: Array<TypeInfo> },
    Reference { definition: DefinitionId, arguments: Array<TypeArgumentInfo> },
    Parameter(GenericParameterRef),
    Tuple(Array<TypeInfo>),
    Record(Array<TypeFieldInfo>),
    Nominal {
        definition: DefinitionId,
        parameters: Array<GenericParameterRef>,
        representation: Option<TypeInfo>,
    },
    Alias {
        definition: DefinitionId,
        parameters: Array<GenericParameterRef>,
        target: TypeInfo,
    },
    Enum {
        definition: DefinitionId,
        parameters: Array<GenericParameterRef>,
        variants: Array<TypeVariantInfo>,
    },
    Applied { constructor: TypeInfo, arguments: Array<TypeArgumentInfo> },
    Function(SignatureInfo),
    Handler {
        handled: EffectRowInfo,
        produced: Option<EffectRowInfo>,
        result: Option<TypeInfo>,
    },
    Existential(Array<SpecInfo>),
    AgentContext(CallableId),
    Task { output: TypeInfo, effects: EffectRowInfo },
}

public enum TypeArgumentInfo {
    Type(TypeInfo),
    Effect(EffectRowInfo),
    Static(StaticValueInfo),
}

public type TypeFieldInfo = { name: string, ty: TypeInfo };

public enum VariantPayloadInfo {
    None,
    Positional(Array<TypeInfo>),
    Named(Array<TypeFieldInfo>),
}

public type TypeVariantInfo = { name: string, payload: VariantPayloadInfo };

public enum EffectTargetInfo {
    Effect(DefinitionId),
    Action { effect: DefinitionId, name: string },
}

public type EffectInfo = {
    target: EffectTargetInfo,
    arguments: Array<StaticValueInfo>,
};

public type EffectRowInfo = {
    effects: Array<EffectInfo>,
    tail: Option<GenericParameterRef>,
};

public type ActionSummary = {
    actions: Array<EffectInfo>,
    tail: Option<GenericParameterRef>,
};

public enum Determinism {
    Deterministic,
    RuntimeMediated,
    NonDeterministic,
}

public enum StaticValueInfo {
    Unit,
    Bool(bool),
    Integer(string),
    Float(string),
    String(string),
    Character(char),
    Type(TypeInfo),
    TypeFingerprint(string),
    Reference(DefinitionId),
    Module { package: string, path: Array<string> },
    Callable(CallableId),
    Resource { root: DefinitionId, fields: Array<string> },
    Wildcard,
    Array(Array<StaticValueInfo>),
    List(List<StaticValueInfo>),
    Set(Array<StaticValueInfo>),
    Tuple(Array<StaticValueInfo>),
    Record(Array<(string, StaticValueInfo)>),
    Constructor {
        kind: string,
        target: StaticValueInfo,
        arguments: Array<AnnotationArgumentInfo>,
    },
}

public enum DescriptorDetail {
    Type(TypeInfo),
    EffectRow(EffectRowInfo),
    Spec(SpecInfo),
    Trace(TraceSummary),
}

public type TraceConstraintInfo = {
    alternatives: Array<Array<TraceRuleInfo>>,
};

public enum TraceRuleInfo {
    Allow(TracePatternInfo),
    Deny(TracePatternInfo),
    RequireBefore { guard: TracePatternInfo, target: TracePatternInfo },
    RequireAfter { target: TracePatternInfo, obligation: TracePatternInfo },
    Limit { count: usize },
}

public type TracePatternInfo = {
    row: EffectRowInfo,
    bounds: Array<TraceSelectorBoundInfo>,
};

public type TraceSelectorBoundInfo = {
    target: EffectTargetInfo,
    argument: usize,
    spec: SpecInfo,
};

public enum TraceExpression {
    Empty,
    Action(EffectInfo),
    Sequence(Array<TraceExpression>),
    Choice(Array<TraceExpression>),
    Parallel(Array<TraceExpression>),
    Repeat(TraceExpression),
    Recursive { identity: string, body: Option<TraceExpression> },
    Reference(DefinitionId),
    Outcome(CallableOutcomeInfo),
    ParameterCall(ParameterCallInfo),
    UnknownOrder(EffectRowInfo),
    Widened { actions: EffectRowInfo, parameter_calls: Array<ParameterCallInfo> },
}

public enum ParameterArgumentInfo {
    Bool(bool),
    Integer { negative: bool, magnitude: string },
    Input { position: usize },
}

public enum ReturnObservationInfo {
    Any,
    Unit,
    Bool(bool),
    Result(bool),
    Option(bool),
    Variant { owner: DefinitionId, name: string },
}

public type ParameterCallInfo = {
    callable: CallableId,
    signature: TypeInfo,
    position: usize,
    projection: Array<ValueProjectionStep>,
    arguments: Option<Array<ParameterArgumentInfo>>,
    handlers: Option<Array<ParameterHandlerScopeInfo>>,
};

// A position inside a checked value; these steps carry no execution authority.
public enum ValueProjectionStep {
    Field(string),
    TupleElement(usize),
    VariantPayload { variant: string, position: usize },
    SequenceElement,
    ListHead,
    ListTail,
    MapKey,
    MapValue,
    CallResult,
    CallWithArguments(usize),
    Invocation(usize),
    Repeat(Array<Array<ValueProjectionStep>>),
}

public enum ParameterHandlerScopeInfo {
    Handle(Array<ParameterHandlerArmInfo>),
}

public type ParameterHandlerArmInfo = {
    action: EffectInfo,
    trace: TraceExpression,
    completion: ParameterHandlerCompletionInfo,
};

public enum ParameterHandlerCompletionInfo {
    Resume { result_type: TypeInfo, value: ReturnObservationInfo },
    Finish { result_type: TypeInfo, value: ReturnObservationInfo },
}

public enum CallableOutcomeInfo {
    Returned { ty: TypeInfo, observation: ReturnObservationInfo },
    Raised { error: TypeInfo },
    NonReturn,
    ParameterNonReturn(ParameterCallInfo),
    ParameterNonReturnTemplate(ParameterCallInfo),
}

public type TraceSummary = {
    expression: TraceExpression,
    complete: bool,
    unresolved: Array<TraceGapInfo>,
};

public type TraceGapInfo = {
    callable: Option<CallableId>,
    position: Option<usize>,
    location: Option<DiagnosticLocation>,
    reason: string,
};

public type TraceWitness = {
    path: Array<TraceWitnessStep>,
};

// This is a coordinate in the described trace graph, not execution authority.
public type TraceWitnessStep = { descriptor_position: usize, label: TraceLabelInfo };

public enum TraceLabelInfo {
    Action(EffectInfo),
    ParameterCall {
        callable: CallableId,
        position: usize,
        projection: Array<ValueProjectionStep>,
        signature: string,
        arguments: Option<Array<ParameterArgumentInfo>>,
        handlers: Option<Array<HandlerScopeLabelInfo>>,
    },
    Returned { ty: string, observation: ReturnObservationInfo },
    Raised { error: string },
    NonReturn,
    ParameterNonReturn {
        callable: CallableId,
        position: usize,
        projection: Array<ValueProjectionStep>,
        signature: string,
        arguments: Option<Array<ParameterArgumentInfo>>,
        handlers: Option<Array<HandlerScopeLabelInfo>>,
    },
}

public enum HandlerScopeLabelInfo {
    Handle(Array<HandlerArmLabelInfo>),
}

public type HandlerArmLabelInfo = {
    action: EffectInfo,
    trace: string,
    control: HandlerControlLabelInfo,
    result_type: string,
    value: ReturnObservationInfo,
};

public enum HandlerControlLabelInfo {
    Resume,
    Finish,
}
