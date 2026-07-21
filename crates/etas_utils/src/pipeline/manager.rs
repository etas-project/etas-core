use std::time::Instant;

use super::{
    Analysis, AnalysisCache, ArtifactKey, ArtifactRef, ArtifactScope, ArtifactSet, PassContext,
    PassControl, PassDescriptor, PassFailure, PassInstrumentation, PassResult, PassRunRecord,
    PassScope, PassTiming, Pipeline, PipelineConfig, PipelineRunResult, PipelineStats,
    PipelineStep, PreservedArtifacts, UnitKey, UnitProvider,
};

pub struct PassManager<C> {
    analyses: AnalysisCache,
    instrumentation: Vec<Box<dyn PassInstrumentation<C>>>,
    config: PipelineConfig,
    artifacts: ArtifactSet,
    records: Vec<PassRunRecord>,
    stats: PipelineStats,
    current_unit: Option<UnitKey>,
}

impl<C> Default for PassManager<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C> PassManager<C> {
    pub fn new() -> Self {
        Self {
            analyses: AnalysisCache::new(),
            instrumentation: Vec::new(),
            config: PipelineConfig::default(),
            artifacts: ArtifactSet::new(),
            records: Vec::new(),
            stats: PipelineStats::default(),
            current_unit: None,
        }
    }

    pub fn with_config(config: PipelineConfig) -> Self {
        Self {
            config,
            ..Self::new()
        }
    }

    pub fn config(&self) -> &PipelineConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut PipelineConfig {
        &mut self.config
    }

    pub fn analyses(&self) -> &AnalysisCache {
        &self.analyses
    }

    pub fn analyses_mut(&mut self) -> &mut AnalysisCache {
        &mut self.analyses
    }

    pub fn analysis<Cx, A>(&mut self, analysis: &A, context: &Cx) -> &A::Output
    where
        A: Analysis<Cx>,
    {
        self.analyses.get_or_run(analysis, context)
    }

    pub fn add_instrumentation<I>(&mut self, instrumentation: I)
    where
        I: PassInstrumentation<C> + 'static,
    {
        self.instrumentation.push(Box::new(instrumentation));
    }

    pub fn available_artifacts(&self) -> &ArtifactSet {
        &self.artifacts
    }

    pub fn mark_artifact_available(&mut self, key: ArtifactKey) {
        self.artifacts.insert(key);
    }

    pub fn mark_artifact_ref_available(&mut self, artifact: ArtifactRef) {
        self.artifacts.insert_ref(artifact);
    }

    pub fn mark_artifacts_available(&mut self, artifacts: &ArtifactSet) {
        self.artifacts.extend(artifacts);
    }

    pub fn run_pipeline(&mut self, pipeline: &mut Pipeline<C>, context: &mut C) -> PipelineRunResult
    where
        C: UnitProvider,
    {
        self.records.clear();
        self.stats = PipelineStats::default();
        self.current_unit = None;

        let control = self.run_steps(&mut pipeline.steps, context);
        let stats = self.stats.clone();
        let records = self.records.clone();
        match control {
            PassControl::Continue => PipelineRunResult::completed(stats, records),
            other => PipelineRunResult::stopped(other, stats, records),
        }
    }

    pub fn run_global_pipeline(
        &mut self,
        pipeline: &mut Pipeline<C>,
        context: &mut C,
    ) -> PipelineRunResult {
        self.records.clear();
        self.stats = PipelineStats::default();
        self.current_unit = None;

        let control = self.run_global_steps(&mut pipeline.steps, context);
        let stats = self.stats.clone();
        let records = self.records.clone();
        match control {
            PassControl::Continue => PipelineRunResult::completed(stats, records),
            other => PipelineRunResult::stopped(other, stats, records),
        }
    }

    pub fn schedule_text(&self, pipeline: &Pipeline<C>) -> String {
        let mut lines = Vec::new();
        Self::collect_schedule(&mut lines, 0, pipeline);
        lines.join("\n")
    }

    fn collect_schedule(lines: &mut Vec<String>, indent: usize, pipeline: &Pipeline<C>) {
        lines.push(format!("{}group {}", "  ".repeat(indent), pipeline.name));
        for step in &pipeline.steps {
            match step {
                PipelineStep::Pass(pass) => {
                    lines.push(format!(
                        "{}pass {}",
                        "  ".repeat(indent + 1),
                        pass.descriptor().name
                    ));
                }
                PipelineStep::Group(group) => Self::collect_schedule(lines, indent + 1, group),
                PipelineStep::ForEach(adapter) => {
                    lines.push(format!(
                        "{}foreach {:?} {:?}",
                        "  ".repeat(indent + 1),
                        adapter.selector,
                        adapter.order
                    ));
                    Self::collect_schedule(lines, indent + 2, &adapter.pipeline);
                }
            }
        }
    }

    fn run_steps(&mut self, steps: &mut [PipelineStep<C>], context: &mut C) -> PassControl
    where
        C: UnitProvider,
    {
        for step in steps {
            let control = match step {
                PipelineStep::Pass(pass) => {
                    let descriptor = pass.descriptor();
                    self.run_pass(pass.as_mut(), descriptor, context)
                }
                PipelineStep::Group(group) => self.run_steps(&mut group.steps, context),
                PipelineStep::ForEach(adapter) => {
                    let units = context.units(&adapter.selector, adapter.order);
                    let previous = self.current_unit;
                    let mut control = PassControl::Continue;
                    for unit in units {
                        self.current_unit = Some(unit);
                        control = self.run_steps(&mut adapter.pipeline.steps, context);
                        if !matches!(control, PassControl::Continue) {
                            break;
                        }
                    }
                    self.current_unit = previous;
                    control
                }
            };

            match control {
                PassControl::Continue => {}
                other => return other,
            }
        }
        PassControl::Continue
    }

    fn run_global_steps(&mut self, steps: &mut [PipelineStep<C>], context: &mut C) -> PassControl {
        for step in steps {
            let control = match step {
                PipelineStep::Pass(pass) => {
                    let descriptor = pass.descriptor();
                    self.run_pass(pass.as_mut(), descriptor, context)
                }
                PipelineStep::Group(group) => self.run_global_steps(&mut group.steps, context),
                PipelineStep::ForEach(adapter) => PassControl::Failed(PassFailure::new(format!(
                    "global pipeline cannot execute foreach {:?} {:?}",
                    adapter.selector, adapter.order
                ))),
            };

            match control {
                PassControl::Continue => {}
                other => return other,
            }
        }
        PassControl::Continue
    }

    fn run_pass(
        &mut self,
        pass: &mut dyn super::Pass<C>,
        descriptor: PassDescriptor,
        context: &mut C,
    ) -> PassControl {
        let pass_context = PassContext::new(self.current_unit);
        if !self.config.should_run(descriptor.name) {
            self.stats.skipped += 1;
            self.records.push(PassRunRecord {
                pass: descriptor.name,
                unit: self.current_unit,
                control: PassControl::Continue,
                changed: false,
                skipped: true,
                produced: ArtifactSet::new(),
                invalidated: ArtifactSet::new(),
                timing: None,
            });
            return PassControl::Continue;
        }

        if !self.scope_matches(descriptor.scope) {
            let result = PassResult {
                control: PassControl::Failed(PassFailure::scope_mismatch(
                    descriptor.name,
                    descriptor.scope,
                    self.current_unit,
                )),
                changed: false,
                preserved: PreservedArtifacts::All,
                produced: ArtifactSet::new(),
            };
            return self.finish_pass(
                &descriptor,
                &pass_context,
                result,
                ArtifactSet::new(),
                None,
                context,
            );
        }

        if self.config.should_stop_before(descriptor.name) {
            self.stats.stopped += 1;
            return PassControl::Stop;
        }

        if let Some(missing) = descriptor.requires.iter_refs().find_map(|artifact| {
            let Some(resolved) = self.resolve_artifact_ref(artifact) else {
                return Some(artifact);
            };
            (!self.artifacts.contains_ref(resolved)).then_some(resolved)
        }) {
            let result = PassResult {
                control: PassControl::Failed(PassFailure::missing_artifact_ref(missing)),
                changed: false,
                preserved: PreservedArtifacts::All,
                produced: ArtifactSet::new(),
            };
            return self.finish_pass(
                &descriptor,
                &pass_context,
                result,
                ArtifactSet::new(),
                None,
                context,
            );
        }

        for hook in &mut self.instrumentation {
            hook.before_pass(&descriptor, &pass_context, context);
        }

        let started = self.config.collect_timing.then(Instant::now);
        let mut result = pass.run(context, &pass_context, self);
        result.produced = self.resolve_artifact_set(&result.produced);
        let produced = self.resolve_artifact_set(&descriptor.produces);
        result.produced.extend(&produced);
        let duration = started.map(|started| started.elapsed());

        let invalidated = if result.changed {
            self.invalidate_after_change(&result.preserved, &result.produced)
        } else {
            ArtifactSet::new()
        };
        self.artifacts.extend(&result.produced);

        self.finish_pass(
            &descriptor,
            &pass_context,
            result,
            invalidated,
            duration,
            context,
        )
    }

    fn finish_pass(
        &mut self,
        descriptor: &PassDescriptor,
        pass_context: &PassContext<C>,
        result: PassResult,
        invalidated: ArtifactSet,
        duration: Option<std::time::Duration>,
        context: &C,
    ) -> PassControl {
        for hook in &mut self.instrumentation {
            hook.after_pass(descriptor, pass_context, &result, context);
        }
        if !invalidated.is_empty() {
            for hook in &mut self.instrumentation {
                hook.after_invalidation(&invalidated);
            }
        }

        self.stats.executed += 1;
        if result.changed {
            self.stats.changed += 1;
        }
        match &result.control {
            PassControl::Continue => {}
            PassControl::Stop => self.stats.stopped += 1,
            PassControl::Failed(_) => self.stats.failed += 1,
        }

        let control = result.control.clone();
        self.records.push(PassRunRecord {
            pass: descriptor.name,
            unit: pass_context.current_unit,
            control: control.clone(),
            changed: result.changed,
            skipped: false,
            produced: result.produced,
            invalidated,
            timing: duration.map(|duration| PassTiming {
                pass: descriptor.name,
                unit: pass_context.current_unit,
                duration,
            }),
        });

        if self.config.should_stop_after(descriptor.name)
            && matches!(control, PassControl::Continue)
        {
            self.stats.stopped += 1;
            return PassControl::Stop;
        }
        control
    }

    fn invalidate_after_change(
        &mut self,
        preserved: &PreservedArtifacts,
        produced: &ArtifactSet,
    ) -> ArtifactSet {
        let analysis_invalidated = self.analyses.invalidate(preserved);
        let artifact_invalidated = match preserved {
            PreservedArtifacts::All => ArtifactSet::new(),
            PreservedArtifacts::None => self.artifacts.difference(produced),
            PreservedArtifacts::Some(keys) => self.artifacts.difference(keys),
        };

        let mut retained = ArtifactSet::new();
        match preserved {
            PreservedArtifacts::All => retained.extend(&self.artifacts),
            PreservedArtifacts::None => {}
            PreservedArtifacts::Some(keys) => {
                for artifact in self
                    .artifacts
                    .iter_refs()
                    .filter(|artifact| keys.contains_ref(*artifact))
                {
                    retained.insert_ref(artifact);
                }
            }
        }
        retained.extend(produced);
        self.artifacts = retained;

        let mut invalidated = artifact_invalidated;
        invalidated.extend(&analysis_invalidated);
        invalidated
    }

    fn scope_matches(&self, scope: PassScope) -> bool {
        match (scope, self.current_unit) {
            (PassScope::Global, None) => true,
            (PassScope::Unit(kind), Some(unit)) => unit.kind == kind,
            _ => false,
        }
    }

    fn resolve_artifact_set(&self, artifacts: &ArtifactSet) -> ArtifactSet {
        artifacts
            .iter_refs()
            .filter_map(|artifact| self.resolve_artifact_ref(artifact))
            .collect()
    }

    fn resolve_artifact_ref(&self, artifact: ArtifactRef) -> Option<ArtifactRef> {
        match artifact.scope {
            ArtifactScope::Global | ArtifactScope::Unit(_) => Some(artifact),
            ArtifactScope::UnitKind(kind) => {
                let current = self.current_unit?;
                (current.kind == kind).then_some(ArtifactRef::unit(artifact.key, current))
            }
        }
    }
}
