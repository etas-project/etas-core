use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    hash::Hash,
    rc::Rc,
};

use etas_utils::graph::UseList;
use etas_utils::{
    Analysis, ArtifactKey, ArtifactRef, ArtifactScope, ArtifactSet, ChainControl,
    ConvergenceStatus, FixpointEngine, Graph, GraphEvent, GraphView, IterationLimit,
    JoinSemiLattice, NodeId, Observable, PartialOrder, Pass, PassContext, PassControl,
    PassDescriptor, PassInstrumentation, PassKind, PassManager, PassResult, PassScope, Pipeline,
    PipelineConfig, PreservedArtifacts, ResponsibilityChain, UnitKey, UnitKindKey, UnitOrder,
    UnitProvider, UnitSelector, ValueChange, ValueChangeKind, Worklist, bfs, cycle_nodes, dfs,
    reverse_postorder, strongly_connected_components, topological_sort,
};

#[derive(Clone, Debug)]
struct TestGraph<N> {
    nodes: Vec<N>,
    edges: BTreeMap<N, Vec<N>>,
}

impl<N> GraphView for TestGraph<N>
where
    N: Clone + Eq + Hash + Ord,
{
    type Node = N;

    fn nodes(&self) -> Vec<Self::Node> {
        self.nodes.clone()
    }

    fn successors(&self, node: &Self::Node) -> Vec<Self::Node> {
        self.edges.get(node).cloned().unwrap_or_default()
    }
}

fn graph(edges: &[(&'static str, &[&'static str])]) -> TestGraph<&'static str> {
    let mut nodes = BTreeSet::new();
    let mut map = BTreeMap::new();

    for (from, successors) in edges {
        nodes.insert(*from);
        for successor in *successors {
            nodes.insert(*successor);
        }
        map.insert(*from, successors.to_vec());
    }

    TestGraph {
        nodes: nodes.into_iter().collect(),
        edges: map,
    }
}

#[test]
fn worklist_deduplicates_queued_nodes() {
    let mut worklist = Worklist::new();

    assert!(worklist.push("a"));
    assert!(!worklist.push("a"));
    assert!(worklist.push("b"));
    assert_eq!(worklist.len(), 2);
    assert_eq!(worklist.pop(), Some("a"));
    assert!(worklist.push("a"));
    assert_eq!(worklist.pop(), Some("b"));
    assert_eq!(worklist.pop(), Some("a"));
    assert!(worklist.is_empty());
}

#[test]
fn lattice_traits_cover_boolean_and_set_join() {
    assert!(false.less_equal(&true));
    assert!(!true.less_equal(&false));

    let mut seen = BTreeSet::from(["a"]);
    assert!(seen.join_assign(&BTreeSet::from(["b"])));
    assert!(!seen.join_assign(&BTreeSet::from(["a", "b"])));
    assert_eq!(seen, BTreeSet::from(["a", "b"]));
}

#[test]
fn fixpoint_engine_reports_convergence_and_iteration_limits() {
    let engine = FixpointEngine::default();
    let result = engine.solve(0, |value| {
        if *value < 3 {
            *value += 1;
            true
        } else {
            false
        }
    });

    assert!(result.converged());
    assert_eq!(result.value, 3);
    assert_eq!(result.stats.changes, 3);

    let limited = FixpointEngine::new(IterationLimit::new(2)).solve(0, |value| {
        *value += 1;
        true
    });
    assert_eq!(limited.status, ConvergenceStatus::IterationLimitReached);
    assert_eq!(limited.value, 2);
}

#[test]
fn worklist_fixpoint_reschedules_successors_on_change() {
    let g = graph(&[("a", &["b"]), ("b", &["c"]), ("c", &[])]);
    let engine = FixpointEngine::default();
    let result = engine.solve_worklist(
        BTreeSet::new(),
        ["a"],
        |node, visited| visited.insert(*node),
        |node, _| g.successors(node),
    );

    assert!(result.converged());
    assert_eq!(result.value, BTreeSet::from(["a", "b", "c"]));
}

#[test]
fn traversal_algorithms_walk_reachable_nodes() {
    let g = graph(&[("a", &["b", "c"]), ("b", &["d"]), ("c", &[]), ("d", &[])]);

    assert_eq!(dfs(&g, ["a"]), vec!["a", "b", "d", "c"]);
    assert_eq!(bfs(&g, ["a"]), vec!["a", "b", "c", "d"]);

    let rpo = reverse_postorder(&g, ["a"]);
    assert_eq!(rpo.first(), Some(&"a"));
    assert!(rpo.iter().all(|node| ["a", "b", "c", "d"].contains(node)));
}

#[test]
fn topological_sort_orders_dags_and_reports_cycles() {
    let dag = graph(&[("a", &["b", "c"]), ("b", &["d"]), ("c", &["d"]), ("d", &[])]);
    let sorted = topological_sort(&dag).expect("dag should sort");
    let pos = |node| sorted.iter().position(|item| item == &node).unwrap();

    assert!(pos("a") < pos("b"));
    assert!(pos("a") < pos("c"));
    assert!(pos("b") < pos("d"));
    assert!(pos("c") < pos("d"));

    let cyclic = graph(&[("a", &["b"]), ("b", &["c"]), ("c", &["a"])]);
    let cycle = cycle_nodes(&cyclic).expect("expected a cycle");
    assert!(cycle.contains(&"a"));
    assert!(cycle.contains(&"b"));
    assert!(cycle.contains(&"c"));
}

#[test]
fn strongly_connected_components_group_cycles() {
    let g = graph(&[
        ("a", &["b"]),
        ("b", &["a", "c"]),
        ("c", &["d"]),
        ("d", &["c"]),
        ("e", &[]),
    ]);
    let components = strongly_connected_components(&g);
    let mut normalized = components
        .into_iter()
        .map(|component| BTreeSet::from_iter(component.nodes))
        .collect::<Vec<_>>();
    normalized.sort_by_key(|component| component.iter().next().copied());

    assert!(normalized.contains(&BTreeSet::from(["a", "b"])));
    assert!(normalized.contains(&BTreeSet::from(["c", "d"])));
    assert!(normalized.contains(&BTreeSet::from(["e"])));
}

#[test]
fn explicit_graph_models_nodes_edges_and_stable_edge_lifecycle() {
    let mut graph = Graph::new();
    let entry = graph.add_node("entry");
    let branch = graph.add_node("branch");
    let exit = graph.add_node("exit");

    let entry_to_branch = graph.add_edge(entry, branch, "cond").expect("valid edge");
    let entry_to_exit = graph
        .add_edge(entry, exit, "fallthrough")
        .expect("valid edge");
    let branch_to_exit = graph.add_edge(branch, exit, "jump").expect("valid edge");

    assert_eq!(graph.node_count(), 3);
    assert_eq!(graph.edge_count(), 3);
    assert_eq!(graph.endpoints(entry_to_branch), Some((entry, branch)));
    assert_eq!(graph.edge_value(entry_to_exit), Some(&"fallthrough"));
    assert_eq!(graph.successors(entry), vec![exit, branch]);
    assert_eq!(graph.predecessors(exit), vec![branch, entry]);

    *graph.edge_value_mut(entry_to_exit).unwrap() = "fast-path";
    assert_eq!(graph.edge_value(entry_to_exit), Some(&"fast-path"));

    let removed = graph
        .remove_edge(entry_to_exit)
        .expect("edge should be live");
    assert_eq!(removed.value, "fast-path");
    assert!(!graph.contains_edge(entry_to_exit));
    assert_eq!(graph.successors(entry), vec![branch]);
    assert_eq!(graph.incoming_edges(exit), vec![branch_to_exit]);

    let sorted = topological_sort(&graph).expect("graph is acyclic");
    let pos = |node| sorted.iter().position(|item| item == &node).unwrap();
    assert!(pos(entry) < pos(branch));
    assert!(pos(branch) < pos(exit));

    assert!(graph.add_edge(NodeId(999), exit, "bad").is_none());
}

#[test]
fn explicit_graph_notifies_observers_around_edge_lifecycle() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let observed_events = Rc::clone(&events);
    let mut graph = Graph::new();
    graph.observe(move |graph: &Graph<&str, &str>, event: &GraphEvent| {
        match event {
            GraphEvent::EdgeRemoving { edge, .. } => {
                assert!(graph.contains_edge(*edge));
            }
            GraphEvent::EdgeRemoved { edge, .. } => {
                assert!(!graph.contains_edge(*edge));
            }
            _ => {}
        }
        observed_events.borrow_mut().push(*event);
    });

    let a = graph.add_node("a");
    let b = graph.add_node("b");
    let edge = graph.add_edge(a, b, "edge").expect("valid edge");
    graph.remove_edge(edge).expect("edge should be removable");

    assert_eq!(
        *events.borrow(),
        vec![
            GraphEvent::NodeAdded { node: a },
            GraphEvent::NodeAdded { node: b },
            GraphEvent::EdgeAdded {
                edge,
                source: a,
                target: b,
            },
            GraphEvent::EdgeRemoving {
                edge,
                source: a,
                target: b,
            },
            GraphEvent::EdgeRemoved {
                edge,
                source: a,
                target: b,
            },
        ]
    );
}

#[test]
fn observable_values_notify_registered_observers() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let observed_events = Rc::clone(&events);
    let mut value = Observable::new(String::from("initial"));

    value.observe(move |current: &String, event: &ValueChange| {
        observed_events
            .borrow_mut()
            .push((current.clone(), event.kind));
    });

    value.update(&ValueChange::new(ValueChangeKind::Updated), |current| {
        current.push_str("-updated");
    });
    let old = value.replace(
        String::from("replacement"),
        &ValueChange::new(ValueChangeKind::Replaced),
    );

    assert_eq!(old, "initial-updated");
    assert_eq!(value.value(), "replacement");
    assert_eq!(
        *events.borrow(),
        vec![
            (String::from("initial-updated"), ValueChangeKind::Updated),
            (String::from("replacement"), ValueChangeKind::Replaced),
        ]
    );
}

#[test]
fn responsibility_chain_stops_after_breaking_step() {
    let mut chain = ResponsibilityChain::new();
    chain.push(|log: &mut Vec<&'static str>| {
        log.push("parse");
        ChainControl::Continue
    });
    chain.push(|log: &mut Vec<&'static str>| {
        log.push("type-check-failed");
        ChainControl::Break
    });
    chain.push(|log: &mut Vec<&'static str>| {
        log.push("lower-air");
        ChainControl::Continue
    });

    let mut log = Vec::new();
    assert_eq!(chain.run(&mut log), ChainControl::Break);
    assert_eq!(log, vec!["parse", "type-check-failed"]);
}

#[test]
fn use_list_tracks_user_operand_slots_and_operand_uses() {
    let mut uses = UseList::new();
    let old = uses.add_operand("old");
    let new = uses.add_operand("new");
    let other = uses.add_operand("other");
    let add = uses.add_user("add");
    let mul = uses.add_user("mul");

    let add_lhs = uses.append_use(add, old).expect("valid use");
    let add_rhs = uses.append_use(add, other).expect("valid use");
    let mul_arg = uses.append_use(mul, old).expect("valid use");

    assert_eq!(uses.user_operands(add), Some(vec![old, other]));
    assert_eq!(uses.operand_uses(old), Some(vec![mul_arg, add_lhs]));
    assert_eq!(uses.operand_uses(other), Some(vec![add_rhs]));

    assert_eq!(uses.set_user_operand(add, 0, new), Some(old));
    assert_eq!(uses.user_operands(add), Some(vec![new, other]));
    assert_eq!(uses.operand_uses(old), Some(vec![mul_arg]));
    assert_eq!(uses.operand_uses(new), Some(vec![add_lhs]));
}

#[test]
fn use_list_replace_all_uses_with_moves_only_existing_uses() {
    let mut uses = UseList::new();
    let old = uses.add_operand("old");
    let replacement = uses.add_operand("replacement");
    let untouched = uses.add_operand("untouched");
    let a = uses.add_user("a");
    let b = uses.add_user("b");
    let c = uses.add_user("c");

    let a_use = uses.append_use(a, old).expect("valid use");
    let b_use = uses.append_use(b, old).expect("valid use");
    let c_use = uses.append_use(c, untouched).expect("valid use");

    assert_eq!(uses.replace_all_uses_with(old, replacement), Some(2));
    assert_eq!(uses.operand_uses(old), Some(vec![]));
    assert_eq!(uses.operand_uses(replacement), Some(vec![a_use, b_use]));
    assert_eq!(uses.operand_uses(untouched), Some(vec![c_use]));
    assert_eq!(uses.user_operands(a), Some(vec![replacement]));
    assert_eq!(uses.user_operands(b), Some(vec![replacement]));
    assert_eq!(uses.user_operands(c), Some(vec![untouched]));
}

#[test]
fn use_list_removal_keeps_slots_stable_and_blocks_live_operand_removal() {
    let mut uses = UseList::new();
    let operand = uses.add_operand("operand");
    let user = uses.add_user("user");
    let use_id = uses.append_use(user, operand).expect("valid use");

    assert!(uses.remove_operand(operand).is_none());
    let removed = uses.remove_use(use_id).expect("use should be removable");

    assert_eq!(removed.user, user);
    assert_eq!(removed.operand, operand);
    assert_eq!(uses.user(user).unwrap().operand_slot_count(), 1);
    assert_eq!(uses.user_uses(user), Some(vec![]));
    assert_eq!(uses.operand_uses(operand), Some(vec![]));
    assert!(uses.remove_operand(operand).is_some());
}

const PARSE: ArtifactKey = ArtifactKey::new("frontend", "parse");
const HIR: ArtifactKey = ArtifactKey::new("frontend", "hir");
const TYPES: ArtifactKey = ArtifactKey::new("frontend", "types");
const BODY: UnitKindKey = UnitKindKey::new("test", "body");

#[derive(Default)]
struct PipelineContext {
    log: Vec<&'static str>,
    units: Vec<UnitKey>,
    analysis_runs: u32,
}

impl UnitProvider for PipelineContext {
    fn units(&self, selector: &UnitSelector, _order: UnitOrder) -> Vec<UnitKey> {
        match selector {
            UnitSelector::Kind(kind) if *kind == BODY => self.units.clone(),
            UnitSelector::Affected(kind) if *kind == BODY => self.units.clone(),
            UnitSelector::AffectedArtifact {
                kind,
                artifact,
                filter: _,
            } if *kind == BODY && *artifact == TYPES => self.units.clone(),
            _ => Vec::new(),
        }
    }
}

struct TestPass {
    name: &'static str,
    scope: PassScope,
    requires: ArtifactSet,
    produces: ArtifactSet,
    preserved: PreservedArtifacts,
    changed: bool,
    control: PassControl,
}

impl TestPass {
    fn new(name: &'static str) -> Self {
        Self {
            name,
            scope: PassScope::Global,
            requires: ArtifactSet::new(),
            produces: ArtifactSet::new(),
            preserved: PreservedArtifacts::All,
            changed: false,
            control: PassControl::Continue,
        }
    }

    fn requires(mut self, artifacts: ArtifactSet) -> Self {
        self.requires = artifacts;
        self
    }

    fn produces(mut self, artifacts: ArtifactSet) -> Self {
        self.produces = artifacts;
        self
    }

    fn scope(mut self, scope: PassScope) -> Self {
        self.scope = scope;
        self
    }

    fn changed(mut self, preserved: PreservedArtifacts) -> Self {
        self.changed = true;
        self.preserved = preserved;
        self
    }

    fn stop(mut self) -> Self {
        self.control = PassControl::Stop;
        self
    }
}

impl Pass<PipelineContext> for TestPass {
    fn descriptor(&self) -> PassDescriptor {
        PassDescriptor::new(self.name, PassKind::Transform)
            .scope(self.scope)
            .requires(self.requires.clone())
            .produces(self.produces.clone())
    }

    fn run(
        &mut self,
        context: &mut PipelineContext,
        pass_context: &PassContext<PipelineContext>,
        _manager: &mut PassManager<PipelineContext>,
    ) -> PassResult {
        context.log.push(self.name);
        if let Some(unit) = pass_context.current_unit {
            context.log.push(match unit.id {
                1 => "unit-1",
                2 => "unit-2",
                _ => "unit-other",
            });
        }
        PassResult {
            control: self.control.clone(),
            changed: self.changed,
            preserved: self.preserved.clone(),
            produced: self.produces.clone(),
        }
    }
}

struct LogInstrumentation(Rc<RefCell<Vec<&'static str>>>);

impl PassInstrumentation<PipelineContext> for LogInstrumentation {
    fn before_pass(
        &mut self,
        pass: &PassDescriptor,
        _pass_context: &PassContext<PipelineContext>,
        _context: &PipelineContext,
    ) {
        self.0.borrow_mut().push(pass.name);
    }

    fn after_invalidation(&mut self, invalidated: &ArtifactSet) {
        if invalidated.contains(PARSE) {
            self.0.borrow_mut().push("invalidated-parse");
        }
    }
}

struct CountAnalysis;

impl Analysis<PipelineContext> for CountAnalysis {
    type Output = u32;

    fn key(&self) -> ArtifactKey {
        TYPES
    }

    fn run(&self, context: &PipelineContext) -> Self::Output {
        context.analysis_runs + 1
    }
}

#[derive(Default)]
struct GlobalPipelineContext {
    log: Vec<&'static str>,
}

struct GlobalTestPass(&'static str);

impl Pass<GlobalPipelineContext> for GlobalTestPass {
    fn descriptor(&self) -> PassDescriptor {
        PassDescriptor::new(self.0, PassKind::Transform)
    }

    fn run(
        &mut self,
        context: &mut GlobalPipelineContext,
        pass_context: &PassContext<GlobalPipelineContext>,
        _manager: &mut PassManager<GlobalPipelineContext>,
    ) -> PassResult {
        assert!(pass_context.current_unit.is_none());
        context.log.push(self.0);
        PassResult::unchanged()
    }
}

#[test]
fn pass_manager_runs_global_pipeline_without_unit_provider() {
    let mut pipeline = Pipeline::new("effects")
        .pass(GlobalTestPass("build-registry"))
        .group(Pipeline::new("solve").pass(GlobalTestPass("solve-summaries")));
    let mut context = GlobalPipelineContext::default();
    let mut manager = PassManager::new();

    let result = manager.run_global_pipeline(&mut pipeline, &mut context);

    assert!(matches!(result.control, PassControl::Continue));
    assert_eq!(context.log, vec!["build-registry", "solve-summaries"]);
    assert_eq!(result.stats.executed, 2);
}

#[test]
fn pass_manager_rejects_for_each_in_global_pipeline() {
    let mut pipeline = Pipeline::new("effects").for_each(
        UnitSelector::Kind(BODY),
        UnitOrder::Stable,
        Pipeline::new("body").pass(GlobalTestPass("check-body")),
    );
    let mut context = GlobalPipelineContext::default();
    let mut manager = PassManager::new();

    let result = manager.run_global_pipeline(&mut pipeline, &mut context);

    let PassControl::Failed(failure) = result.control else {
        panic!("expected global pipeline foreach failure");
    };
    assert!(
        failure
            .message
            .contains("global pipeline cannot execute foreach")
    );
    assert!(context.log.is_empty());
}

#[test]
fn pass_manager_runs_nested_pipeline_and_tracks_artifacts() {
    let mut pipeline = Pipeline::new("frontend")
        .pass(TestPass::new("parse").produces(ArtifactSet::one(PARSE)))
        .group(
            Pipeline::new("check")
                .pass(
                    TestPass::new("lower")
                        .requires(ArtifactSet::one(PARSE))
                        .produces(ArtifactSet::one(HIR)),
                )
                .pass(
                    TestPass::new("typecheck")
                        .requires(ArtifactSet::one(HIR))
                        .produces(ArtifactSet::one(TYPES)),
                ),
        );

    let mut context = PipelineContext::default();
    let mut manager = PassManager::new();
    let result = manager.run_pipeline(&mut pipeline, &mut context);

    assert!(
        matches!(result.control, PassControl::Continue),
        "{:?}",
        result.control
    );
    assert_eq!(context.log, vec!["parse", "lower", "typecheck"]);
    assert!(manager.available_artifacts().contains(PARSE));
    assert!(manager.available_artifacts().contains(HIR));
    assert!(manager.available_artifacts().contains(TYPES));
    assert_eq!(result.stats.executed, 3);
    assert_eq!(
        manager.schedule_text(&pipeline),
        "group frontend\n  pass parse\n  group check\n    pass lower\n    pass typecheck"
    );
}

#[test]
fn pass_manager_runs_for_each_adapter_with_current_unit() {
    let mut pipeline = Pipeline::new("frontend").for_each(
        UnitSelector::Kind(BODY),
        UnitOrder::Stable,
        Pipeline::new("body").pass(TestPass::new("check-body").scope(PassScope::Unit(BODY))),
    );
    let mut context = PipelineContext {
        units: vec![UnitKey::new(BODY, 1), UnitKey::new(BODY, 2)],
        ..PipelineContext::default()
    };
    let mut manager = PassManager::with_config(PipelineConfig::default().with_timing(true));

    let result = manager.run_pipeline(&mut pipeline, &mut context);

    assert!(matches!(result.control, PassControl::Continue));
    assert_eq!(
        context.log,
        vec!["check-body", "unit-1", "check-body", "unit-2"]
    );
    assert_eq!(result.records.len(), 2);
    assert_eq!(result.records[0].unit, Some(UnitKey::new(BODY, 1)));
    assert_eq!(result.records[1].unit, Some(UnitKey::new(BODY, 2)));
    assert_eq!(
        result.records[0].timing.as_ref().map(|timing| timing.unit),
        Some(Some(UnitKey::new(BODY, 1)))
    );
    assert_eq!(
        manager.schedule_text(&pipeline),
        "group frontend\n  foreach Kind(UnitKindKey { namespace: \"test\", name: \"body\" }) Stable\n    group body\n      pass check-body"
    );
}

#[test]
fn pass_manager_runs_artifact_aware_affected_for_each_adapter() {
    let mut pipeline = Pipeline::new("frontend").for_each(
        UnitSelector::AffectedArtifact {
            kind: BODY,
            artifact: TYPES,
            filter: None,
        },
        UnitOrder::Stable,
        Pipeline::new("body").pass(TestPass::new("check-body").scope(PassScope::Unit(BODY))),
    );
    let mut context = PipelineContext {
        units: vec![UnitKey::new(BODY, 2)],
        ..PipelineContext::default()
    };
    let mut manager = PassManager::new();

    let result = manager.run_pipeline(&mut pipeline, &mut context);

    assert!(matches!(result.control, PassControl::Continue));
    assert_eq!(context.log, vec!["check-body", "unit-2"]);
    assert_eq!(result.records.len(), 1);
    assert_eq!(result.records[0].unit, Some(UnitKey::new(BODY, 2)));
    assert!(
        manager
            .schedule_text(&pipeline)
            .contains("AffectedArtifact")
    );
}

#[test]
fn pass_manager_rejects_unit_scoped_pass_without_matching_current_unit() {
    let mut pipeline =
        Pipeline::new("frontend").pass(TestPass::new("check-body").scope(PassScope::Unit(BODY)));
    let mut context = PipelineContext::default();
    let mut manager = PassManager::new();

    let result = manager.run_pipeline(&mut pipeline, &mut context);

    let PassControl::Failed(failure) = result.control else {
        panic!("expected scope mismatch failure");
    };
    assert!(failure.message.contains("check-body"));
    assert!(failure.message.contains("current unit is None"));
    assert_eq!(context.log, Vec::<&'static str>::new());
}

#[test]
fn pass_manager_resolves_unit_kind_artifacts_against_current_unit() {
    let unit_1 = UnitKey::new(BODY, 1);
    let unit_2 = UnitKey::new(BODY, 2);
    let unit_hir = ArtifactRef::unit_kind(HIR, BODY);
    let mut pipeline = Pipeline::new("frontend").for_each(
        UnitSelector::Kind(BODY),
        UnitOrder::Stable,
        Pipeline::new("body")
            .pass(
                TestPass::new("lower-body")
                    .scope(PassScope::Unit(BODY))
                    .produces(ArtifactSet::from_iter([unit_hir])),
            )
            .pass(
                TestPass::new("check-body")
                    .scope(PassScope::Unit(BODY))
                    .requires(ArtifactSet::from_iter([unit_hir]))
                    .produces(ArtifactSet::one(TYPES)),
            ),
    );
    let mut context = PipelineContext {
        units: vec![unit_1, unit_2],
        ..PipelineContext::default()
    };
    let mut manager = PassManager::new();

    let result = manager.run_pipeline(&mut pipeline, &mut context);

    assert!(matches!(result.control, PassControl::Continue));
    assert!(
        manager
            .available_artifacts()
            .contains_ref(ArtifactRef::unit(HIR, unit_1))
    );
    assert!(
        manager
            .available_artifacts()
            .contains_ref(ArtifactRef::unit(HIR, unit_2))
    );
    assert!(!manager.available_artifacts().contains(HIR));
    assert!(manager.available_artifacts().contains(TYPES));
    assert_eq!(
        context.log,
        vec![
            "lower-body",
            "unit-1",
            "check-body",
            "unit-1",
            "lower-body",
            "unit-2",
            "check-body",
            "unit-2"
        ]
    );
}

#[test]
fn artifact_set_iter_includes_scoped_artifacts() {
    let body_unit = UnitKey::new(BODY, 7);
    let artifacts = ArtifactSet::from_iter([
        ArtifactRef::global(PARSE),
        ArtifactRef::unit(HIR, body_unit),
        ArtifactRef::unit_kind(TYPES, BODY),
    ]);

    let collected = artifacts.iter().collect::<Vec<_>>();
    assert!(collected.contains(&ArtifactRef::global(PARSE)));
    assert!(collected.contains(&ArtifactRef::unit(HIR, body_unit)));
    assert!(collected.contains(&ArtifactRef::unit_kind(TYPES, BODY)));

    let keys = artifacts.iter_keys().collect::<Vec<_>>();
    assert!(keys.contains(&PARSE));
    assert!(keys.contains(&HIR));
    assert!(keys.contains(&TYPES));
}

#[test]
fn pass_manager_reports_missing_required_artifacts() {
    let mut pipeline = Pipeline::new("frontend").pass(
        TestPass::new("lower")
            .requires(ArtifactSet::one(PARSE))
            .produces(ArtifactSet::one(HIR)),
    );
    let mut context = PipelineContext::default();
    let mut manager = PassManager::new();
    let result = manager.run_pipeline(&mut pipeline, &mut context);

    let PassControl::Failed(failure) = result.control else {
        panic!("expected missing artifact failure");
    };
    assert_eq!(failure.missing_artifact, Some(PARSE));
    assert_eq!(context.log, Vec::<&'static str>::new());
}

#[test]
fn pass_manager_invalidates_unpreserved_artifacts_and_cached_analyses() {
    let mut pipeline = Pipeline::new("frontend").pass(
        TestPass::new("rewrite")
            .produces(ArtifactSet::one(HIR))
            .changed(PreservedArtifacts::Some(ArtifactSet::one(HIR))),
    );
    let mut context = PipelineContext::default();
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut manager = PassManager::new();
    manager.mark_artifact_available(PARSE);
    manager.mark_artifact_available(HIR);
    manager.analyses_mut().insert(TYPES, 7_u32);
    manager.add_instrumentation(LogInstrumentation(Rc::clone(&events)));

    let result = manager.run_pipeline(&mut pipeline, &mut context);

    assert!(matches!(result.control, PassControl::Continue));
    assert!(!manager.available_artifacts().contains(PARSE));
    assert!(manager.available_artifacts().contains(HIR));
    assert!(!manager.analyses().contains(TYPES));
    assert_eq!(*events.borrow(), vec!["rewrite", "invalidated-parse"]);
}

#[test]
fn pass_descriptor_records_artifact_contract_and_granularity() {
    let descriptor = PassDescriptor::new("check-body", PassKind::Analysis)
        .scope(PassScope::Unit(BODY))
        .requires(ArtifactSet::one(HIR))
        .produces(ArtifactSet::one(TYPES));

    assert_eq!(descriptor.granularity, ArtifactScope::UnitKind(BODY));
    assert!(descriptor.requires.contains(HIR));
    assert!(descriptor.produces.contains(TYPES));
    assert!(descriptor.invalidates.contains(TYPES));

    let descriptor = descriptor.invalidates(ArtifactSet::one(HIR));
    assert!(descriptor.invalidates.contains(HIR));
    assert!(!descriptor.invalidates.contains(TYPES));
}

#[test]
fn pass_manager_supports_filters_stop_and_analysis_cache() {
    let mut pipeline = Pipeline::new("frontend")
        .pass(TestPass::new("parse").produces(ArtifactSet::one(PARSE)))
        .pass(TestPass::new("skip-me"))
        .pass(TestPass::new("stop-here").stop())
        .pass(TestPass::new("after-stop"));

    let mut context = PipelineContext::default();
    let mut manager = PassManager::with_config(PipelineConfig::default().disable("skip-me"));
    let result = manager.run_pipeline(&mut pipeline, &mut context);

    assert!(matches!(result.control, PassControl::Stop));
    assert_eq!(context.log, vec!["parse", "stop-here"]);
    assert_eq!(result.stats.skipped, 1);

    let analysis = CountAnalysis;
    let first = *manager.analysis(&analysis, &context);
    context.analysis_runs = 99;
    let second = *manager.analysis(&analysis, &context);
    assert_eq!(first, 1);
    assert_eq!(second, 1);
}
