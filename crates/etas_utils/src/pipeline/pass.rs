use super::{ArtifactScope, ArtifactSet, PassContext, PassManager, PassResult, PassScope};

pub trait Pass<C> {
    fn descriptor(&self) -> PassDescriptor;
    fn run(
        &mut self,
        context: &mut C,
        pass_context: &PassContext<C>,
        manager: &mut PassManager<C>,
    ) -> PassResult;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PassDescriptor {
    pub name: &'static str,
    pub kind: PassKind,
    pub scope: PassScope,
    pub granularity: ArtifactScope,
    pub requires: ArtifactSet,
    pub produces: ArtifactSet,
    pub invalidates: ArtifactSet,
}

impl PassDescriptor {
    pub fn new(name: &'static str, kind: PassKind) -> Self {
        Self {
            name,
            kind,
            scope: PassScope::Global,
            granularity: ArtifactScope::Global,
            requires: ArtifactSet::new(),
            produces: ArtifactSet::new(),
            invalidates: ArtifactSet::new(),
        }
    }

    pub fn scope(mut self, scope: PassScope) -> Self {
        self.scope = scope;
        self.granularity = match scope {
            PassScope::Global => ArtifactScope::Global,
            PassScope::Unit(kind) => ArtifactScope::UnitKind(kind),
        };
        self
    }

    pub fn granularity(mut self, granularity: ArtifactScope) -> Self {
        self.granularity = granularity;
        self
    }

    pub fn requires(mut self, requires: ArtifactSet) -> Self {
        self.requires = requires;
        self
    }

    pub fn produces(mut self, produces: ArtifactSet) -> Self {
        if self.invalidates.is_empty() {
            self.invalidates = produces.clone();
        }
        self.produces = produces;
        self
    }

    pub fn invalidates(mut self, invalidates: ArtifactSet) -> Self {
        self.invalidates = invalidates;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassKind {
    Transform,
    Analysis,
    Verify,
    Plan,
    Emit,
}
