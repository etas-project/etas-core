pub mod automaton;
pub mod fixpoint;
pub mod graph;
pub mod pattern;
pub mod pipeline;
pub mod profile;

pub use automaton::{
    AcceptingState, AutomatonMonitor, InitialState, Matcher, NoTransitionPolicy, ProductAutomaton,
    ProductState, ProductStepResult, ProductTraceStep, RejectingState, State, StateSet, StepError,
    StepResult, SymbolicAutomaton, TraceStep, Transition,
};
pub use fixpoint::{
    Constraint, ConvergenceStatus, EdgeTransfer, FixpointEngine, FixpointResult, FixpointStats,
    IterationLimit, JoinSemiLattice, Lattice, MeetSemiLattice, NodeTransfer, PartialOrder,
    Transfer, Worklist,
};
pub use graph::{
    Edge, EdgeId, Graph, GraphEvent, GraphView, Node, NodeId, Scc, bfs, cycle_nodes, dfs,
    reverse_postorder, strongly_connected_components, topological_sort,
};
pub use pattern::{
    ChainControl, ChainStep, Observable, Observer, ResponsibilityChain, ValueChange,
    ValueChangeKind,
};
pub use pipeline::{
    Analysis, AnalysisCache, ArtifactKey, ArtifactRef, ArtifactScope, ArtifactSet, ForEachAdapter,
    Pass, PassContext, PassControl, PassDescriptor, PassFailure, PassInstrumentation, PassKind,
    PassManager, PassResult, PassRunRecord, PassScope, PassTiming, Pipeline, PipelineConfig,
    PipelineRunResult, PipelineStats, PipelineStep, PreservedArtifacts, UnitFilterKey, UnitKey,
    UnitKindKey, UnitOrder, UnitProvider, UnitSelector,
};
pub use profile::{
    ProfileCounter, ProfileHandle, ProfileRecorder, ProfileReport, ProfileSpan, ProfileSpanGuard,
    ProfileSpanStatus, ProfileTreeRenderOptions, render_profile_tree,
    render_profile_tree_with_options, write_profile_report,
};
