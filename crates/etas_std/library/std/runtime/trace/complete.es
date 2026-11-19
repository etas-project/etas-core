module std.runtime.trace;

public let Logical: TraceLabel = TraceLabel { tree= TraceTree.Logical };

enum TraceTree {
    Logical,
    Callable(std.compiler.DefinitionId, std.compiler.RevisionInfo),
    Composite(Array<TraceLabel>),
}

public type TraceLabel = { private tree: TraceTree };

public flow VirtualStages(stages: Array<TraceLabel>) -> TraceLabel ![] {
    return TraceLabel { tree= TraceTree.Composite(stages) };
}
