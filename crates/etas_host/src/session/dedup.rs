use crate::{HostError, HostErrorCode, SessionMessage};
pub(super) fn validate_replay(
    existing: &SessionMessage,
    incoming: &SessionMessage,
) -> Result<(), HostError> {
    // Delivery IDs can change on retry; the logical append cannot.
    if existing.session != incoming.session
        || existing.role != incoming.role
        || existing.from != incoming.from
        || existing.to != incoming.to
        || existing.created_at != incoming.created_at
        || existing.payload != incoming.payload
        || existing.provenance != incoming.provenance
    {
        return Err(HostError::new(
            HostErrorCode::InvalidRequest,
            "session dedup key was reused for a different logical append",
        ));
    }
    Ok(())
}
