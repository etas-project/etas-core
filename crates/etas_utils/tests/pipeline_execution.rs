use etas_utils::{
    ArtifactKey, ArtifactSet, Pass, PassContext, PassControl, PassDescriptor, PassKind,
    PassManager, PassResult, Pipeline, PipelineExecutionControl, PipelineInterruption,
    PipelineInterruptionKind,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

const OUTPUT: ArtifactKey = ArtifactKey::new("test", "output");
struct StopAt {
    checks: AtomicUsize,
    limit: usize,
}
impl PipelineExecutionControl for StopAt {
    fn checkpoint(&self) -> Result<(), PipelineInterruption> {
        if self.checks.fetch_add(1, Ordering::SeqCst) >= self.limit {
            Err(PipelineInterruption {
                kind: PipelineInterruptionKind::Cancelled,
                reason: "test cancellation".into(),
            })
        } else {
            Ok(())
        }
    }
}
struct Produce;
impl Pass<usize> for Produce {
    fn descriptor(&self) -> PassDescriptor {
        PassDescriptor::new("produce", PassKind::Emit).produces(ArtifactSet::from([OUTPUT]))
    }
    fn run(
        &mut self,
        context: &mut usize,
        _: &PassContext<usize>,
        _: &mut PassManager<usize>,
    ) -> PassResult {
        *context += 1;
        PassResult::unchanged()
    }
}

#[test]
fn interruption_before_dispatch_does_not_run_or_produce() {
    verify_interruption(0, 0);
}

#[test]
fn interruption_during_a_pass_prevents_artifact_commit_and_later_stages() {
    verify_interruption(1, 1);
}

fn verify_interruption(limit: usize, expected_runs: usize) {
    let mut pipeline = Pipeline::new("controlled").pass(Produce).pass(Produce);
    let mut manager = PassManager::new();
    manager.set_execution_control(Arc::new(StopAt {
        checks: AtomicUsize::new(0),
        limit,
    }));
    let mut runs = 0;
    let result = manager.run_global_pipeline(&mut pipeline, &mut runs);
    let PassControl::Failed(failure) = result.control else {
        panic!("interruption must fail the pipeline")
    };
    assert_eq!(
        failure.interruption.unwrap().kind,
        PipelineInterruptionKind::Cancelled
    );
    assert_eq!(runs, expected_runs);
    assert_eq!(result.records.len(), 1);
    assert!(!manager.available_artifacts().contains(OUTPUT));
    assert!(result.records[0].produced.is_empty());
}

struct Fail;
impl Pass<usize> for Fail {
    fn descriptor(&self) -> PassDescriptor {
        PassDescriptor::new("fail", PassKind::Verify).produces(ArtifactSet::one(OUTPUT))
    }
    fn run(
        &mut self,
        _: &mut usize,
        _: &PassContext<usize>,
        _: &mut PassManager<usize>,
    ) -> PassResult {
        let mut result = PassResult::failed("verification failed");
        result.produced = ArtifactSet::one(ArtifactKey::new("test", "dynamic_output"));
        result
    }
}

#[test]
fn failed_pass_does_not_commit_declared_or_dynamic_outputs() {
    let mut manager = PassManager::new();
    let result =
        manager.run_global_pipeline(&mut Pipeline::new("test").pass(Fail).pass(Produce), &mut 0);
    assert!(matches!(result.control, PassControl::Failed(_)));
    assert!(manager.available_artifacts().is_empty());
    assert!(result.records[0].produced.is_empty());
    assert_eq!(result.records.len(), 1);
}

#[test]
fn failed_revalidation_revokes_a_previously_ready_output() {
    let mut manager = PassManager::new();
    manager.run_global_pipeline(&mut Pipeline::new("first").pass(Produce), &mut 0);
    assert!(manager.available_artifacts().contains(OUTPUT));
    let result = manager.run_global_pipeline(&mut Pipeline::new("revalidate").pass(Fail), &mut 0);
    assert!(matches!(result.control, PassControl::Failed(_)));
    assert!(!manager.available_artifacts().contains(OUTPUT));
    assert!(result.records[0].produced.is_empty());
    assert!(result.records[0].invalidated.contains(OUTPUT));
}
