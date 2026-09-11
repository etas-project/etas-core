use crate::{HostError, HostErrorCode, StorageLimits};

// A reservation includes bounded version/disposition fields and ledger nodes.
// Variable identity strings are charged twice for the lookup key and receipt.
pub(crate) const FIXED_BYTES: usize = 1024;

pub(crate) fn charge<'a>(parts: impl IntoIterator<Item = &'a str>) -> Result<usize, HostError> {
    parts.into_iter().try_fold(FIXED_BYTES, |size, part| {
        part.len()
            .checked_mul(2)
            .and_then(|bytes| size.checked_add(bytes))
            .ok_or_else(exceeded)
    })
}

pub(crate) fn admit(
    limits: &StorageLimits,
    used: usize,
    additional: usize,
) -> Result<(), HostError> {
    if used
        .checked_add(additional)
        .is_none_or(|total| total > limits.max_receipt_bytes)
    {
        Err(exceeded())
    } else {
        Ok(())
    }
}

pub(crate) fn sum(
    charges: impl IntoIterator<Item = Result<usize, HostError>>,
) -> Result<usize, HostError> {
    charges.into_iter().try_fold(0usize, |sum, charge| {
        sum.checked_add(charge?).ok_or_else(exceeded)
    })
}

pub(crate) fn exceeded() -> HostError {
    HostError::new(
        HostErrorCode::BudgetExceeded,
        "storage receipt byte budget exceeded",
    )
}
