use super::{ArtifactSet, PassContext, PassDescriptor, PassResult};

pub trait PassInstrumentation<C> {
    fn before_pass(
        &mut self,
        _pass: &PassDescriptor,
        _pass_context: &PassContext<C>,
        _context: &C,
    ) {
    }

    fn after_pass(
        &mut self,
        _pass: &PassDescriptor,
        _pass_context: &PassContext<C>,
        _result: &PassResult,
        _context: &C,
    ) {
    }

    fn after_invalidation(&mut self, _invalidated: &ArtifactSet) {}
}
