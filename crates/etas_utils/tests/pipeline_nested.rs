use etas_utils::{
    ArtifactKey, ArtifactSet, Pass, PassContext, PassControl, PassDescriptor, PassKind,
    PassManager, PassResult, PassScope, Pipeline, PreservedArtifacts, UnitKey, UnitKindKey,
    UnitOrder, UnitProvider, UnitSelector,
};

const INPUT: ArtifactKey = ArtifactKey::new("nested", "input");
const PARENT: UnitKindKey = UnitKindKey::new("nested", "parent");
const CHILD: UnitKindKey = UnitKindKey::new("nested", "child");

#[derive(Default)]
struct Context(Vec<(&'static str, Option<UnitKey>)>);

impl UnitProvider for Context {
    fn units(&self, _: &UnitSelector, _: UnitOrder) -> Vec<UnitKey> {
        vec![UnitKey::new(PARENT, 7)]
    }
}

struct Stage(&'static str, Option<UnitKindKey>);
impl Pass<Context> for Stage {
    fn descriptor(&self) -> PassDescriptor {
        let descriptor = PassDescriptor::new(self.0, PassKind::Analysis);
        let descriptor = match self.1 {
            Some(kind) => descriptor.scope(PassScope::Unit(kind)),
            None => descriptor,
        };
        if self.0 == "before" {
            descriptor.produces(ArtifactSet::one(INPUT))
        } else {
            descriptor.requires(ArtifactSet::one(INPUT))
        }
    }
    fn run(
        &mut self,
        context: &mut Context,
        pass: &PassContext<Context>,
        _: &mut PassManager<Context>,
    ) -> PassResult {
        context.0.push((self.0, pass.current_unit));
        if self.0 == "before" {
            PassResult::changed(PreservedArtifacts::All, ArtifactSet::one(INPUT))
        } else {
            PassResult::unchanged()
        }
    }
}

struct Nested;
impl Pass<Context> for Nested {
    fn descriptor(&self) -> PassDescriptor {
        PassDescriptor::new("nested", PassKind::Analysis).scope(PassScope::Unit(PARENT))
    }
    fn run(
        &mut self,
        context: &mut Context,
        pass: &PassContext<Context>,
        manager: &mut PassManager<Context>,
    ) -> PassResult {
        context.0.push(("nested", pass.current_unit));
        let mut child = Pipeline::new("child").pass(Stage("child", Some(CHILD)));
        let control =
            manager.run_nested_pipeline(&mut child, context, Some(UnitKey::new(CHILD, 11)));
        PassResult {
            control,
            ..PassResult::unchanged()
        }
    }
}

#[test]
fn nested_pipeline_preserves_artifacts_records_statistics_and_parent_scope() {
    let mut pipeline = Pipeline::new("outer").pass(Stage("before", None)).for_each(
        UnitSelector::Kind(PARENT),
        UnitOrder::Stable,
        Pipeline::new("parent")
            .pass(Nested)
            .pass(Stage("after", Some(PARENT))),
    );
    let mut manager = PassManager::new();
    let mut context = Context::default();
    let result = manager.run_pipeline(&mut pipeline, &mut context);
    assert_eq!(result.control, PassControl::Continue);
    assert_eq!(
        context.0,
        vec![
            ("before", None),
            ("nested", Some(UnitKey::new(PARENT, 7))),
            ("child", Some(UnitKey::new(CHILD, 11))),
            ("after", Some(UnitKey::new(PARENT, 7))),
        ]
    );
    assert_eq!(
        result
            .records
            .iter()
            .map(|record| record.pass)
            .collect::<Vec<_>>(),
        ["before", "child", "nested", "after"]
    );
    assert_eq!(result.stats.executed, 4);
    assert!(manager.available_artifacts().contains(INPUT));
}

#[test]
fn nested_pipeline_propagates_actual_missing_artifact_failure() {
    let mut pipeline = Pipeline::new("outer").for_each(
        UnitSelector::Kind(PARENT),
        UnitOrder::Stable,
        Pipeline::new("parent")
            .pass(Nested)
            .pass(Stage("after", Some(PARENT))),
    );
    let mut manager = PassManager::new();
    let result = manager.run_pipeline(&mut pipeline, &mut Context::default());
    let PassControl::Failed(failure) = result.control else {
        panic!("nested dependency must be enforced")
    };
    assert_eq!(failure.missing_artifact, Some(INPUT));
    assert_eq!(
        result
            .records
            .iter()
            .map(|record| record.pass)
            .collect::<Vec<_>>(),
        ["child", "nested"]
    );
    assert!(!manager.available_artifacts().contains(INPUT));
}
