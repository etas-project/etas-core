use super::*;
impl SessionDatabase<'_> {
    pub(super) fn load(
        &mut self,
        session: SessionRef,
        context: ContextPolicy,
        cursor: Option<SessionCursor>,
        limit: Option<u32>,
    ) -> Result<SessionResult, HostError> {
        super::paging::load(self, session, context, cursor, limit)
    }
}
