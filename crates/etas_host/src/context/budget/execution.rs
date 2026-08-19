use std::{
    fmt,
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use crate::{HostError, HostErrorCode};

use super::{Budget, CostBudget};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionBudgetSnapshot {
    pub deadline_unix_millis: Option<u128>,
    pub reserved_tokens: u64,
    pub consumed_tokens: u64,
    pub reserved_cost_micros: u128,
    pub consumed_cost_micros: u128,
}

#[derive(Clone)]
pub struct ExecutionBudgetState {
    inner: Arc<Mutex<BudgetLedger>>,
}

#[derive(Clone)]
pub struct ExecutionBudget {
    limits: Budget,
    state: ExecutionBudgetState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TokenReservation {
    amount: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CostReservation {
    amount_micros: u128,
    currency: String,
}

#[derive(Debug)]
struct BudgetLedger {
    deadline: Deadline,
    deadline_unix_millis: Option<u128>,
    max_tokens: Option<u64>,
    max_cost_micros: Option<u128>,
    cost_currency: Option<String>,
    reserved_tokens: u64,
    consumed_tokens: u64,
    reserved_cost_micros: u128,
    consumed_cost_micros: u128,
}

#[derive(Debug)]
enum Deadline {
    Unlimited,
    At(Instant),
    Invalid,
}

impl ExecutionBudget {
    pub fn start(limits: Budget) -> Self {
        let state = ExecutionBudgetState::start(&limits);
        Self { limits, state }
    }

    pub fn restore(limits: Budget, snapshot: ExecutionBudgetSnapshot) -> Result<Self, HostError> {
        let state = ExecutionBudgetState::restore(&limits, snapshot)?;
        Ok(Self { limits, state })
    }

    pub fn limits(&self) -> &Budget {
        &self.limits
    }

    pub fn with_limits(&self, limits: Budget) -> Self {
        Self {
            limits,
            state: self.state.clone(),
        }
    }

    pub fn replace_limits(&mut self, limits: Budget) {
        *self = Self::start(limits);
    }

    pub fn state(&self) -> ExecutionBudgetState {
        self.state.clone()
    }

    pub fn rebind_state(&mut self, state: &ExecutionBudgetState) {
        self.state = state.clone();
    }

    pub fn snapshot(&self) -> Result<ExecutionBudgetSnapshot, HostError> {
        self.state.snapshot()
    }

    pub fn check_time(&self) -> Result<(), HostError> {
        self.state.check_time()
    }

    pub fn deadline(&self) -> Result<Option<tokio::time::Instant>, HostError> {
        self.state.deadline()
    }

    pub fn reserve_tokens(&self, amount: u64) -> Result<TokenReservation, HostError> {
        let mut ledger = self.state.lock()?;
        let max = minimum_limit(
            ledger.max_tokens,
            self.limits.tokens.map(|tokens| tokens.max_tokens),
        );
        let next = ledger
            .consumed_tokens
            .checked_add(ledger.reserved_tokens)
            .and_then(|used| used.checked_add(amount))
            .ok_or_else(token_budget_exceeded)?;
        if max.is_some_and(|max| next > max) {
            return Err(token_budget_exceeded());
        }
        ledger.reserved_tokens += amount;
        Ok(TokenReservation { amount })
    }

    pub fn remaining_tokens(&self) -> Result<Option<u64>, HostError> {
        let ledger = self.state.lock()?;
        let max = minimum_limit(
            ledger.max_tokens,
            self.limits.tokens.map(|tokens| tokens.max_tokens),
        );
        let used = ledger
            .consumed_tokens
            .checked_add(ledger.reserved_tokens)
            .ok_or_else(token_budget_exceeded)?;
        Ok(max.map(|max| max.saturating_sub(used)))
    }

    pub fn remaining_cost(&self) -> Result<Option<CostBudget>, HostError> {
        let ledger = self.state.lock()?;
        let local = self.limits.cost.as_ref();
        let max = minimum_limit(ledger.max_cost_micros, local.map(|cost| cost.max_micros));
        let Some(max) = max else {
            return Ok(None);
        };
        let currency = match (ledger.cost_currency.as_deref(), local) {
            (Some(global), Some(local)) if global != local.currency => {
                return Err(HostError::new(
                    HostErrorCode::InvalidRequest,
                    "scoped cost budget currency does not match the run-owned budget",
                )
                .with_detail("global", global.to_owned())
                .with_detail("scoped", local.currency.clone()));
            }
            (Some(global), _) => global.to_owned(),
            (None, Some(local)) => local.currency.clone(),
            (None, None) => {
                return Err(corrupt_budget_state(
                    "cost budget has a limit but no currency identity",
                ));
            }
        };
        let used = ledger
            .consumed_cost_micros
            .checked_add(ledger.reserved_cost_micros)
            .ok_or_else(cost_budget_exceeded)?;
        Ok(Some(CostBudget {
            max_micros: max.saturating_sub(used),
            currency,
        }))
    }

    pub fn settle_tokens(
        &self,
        reservation: TokenReservation,
        consumed: u64,
    ) -> Result<(), HostError> {
        let mut ledger = self.state.lock()?;
        if ledger.reserved_tokens < reservation.amount {
            return Err(corrupt_budget_state(
                "token reservation exceeds the run-owned reserved token count",
            ));
        }
        ledger.reserved_tokens -= reservation.amount;
        ledger.consumed_tokens = ledger
            .consumed_tokens
            .checked_add(consumed)
            .ok_or_else(token_budget_exceeded)?;
        let max = minimum_limit(
            ledger.max_tokens,
            self.limits.tokens.map(|tokens| tokens.max_tokens),
        );
        let committed_and_reserved = ledger
            .consumed_tokens
            .checked_add(ledger.reserved_tokens)
            .ok_or_else(token_budget_exceeded)?;
        if max.is_some_and(|max| committed_and_reserved > max) {
            return Err(token_budget_exceeded());
        }
        Ok(())
    }

    pub fn release_tokens(&self, reservation: TokenReservation) -> Result<(), HostError> {
        let mut ledger = self.state.lock()?;
        if ledger.reserved_tokens < reservation.amount {
            return Err(corrupt_budget_state(
                "token reservation exceeds the run-owned reserved token count",
            ));
        }
        ledger.reserved_tokens -= reservation.amount;
        Ok(())
    }

    pub fn reserve_cost(
        &self,
        amount_micros: u128,
        currency: &str,
    ) -> Result<CostReservation, HostError> {
        let mut ledger = self.state.lock()?;
        validate_currency(&ledger, &self.limits, currency)?;
        let local_max = self.limits.cost.as_ref().map(|cost| cost.max_micros);
        let max = minimum_limit(ledger.max_cost_micros, local_max);
        let next = ledger
            .consumed_cost_micros
            .checked_add(ledger.reserved_cost_micros)
            .and_then(|used| used.checked_add(amount_micros))
            .ok_or_else(cost_budget_exceeded)?;
        if max.is_some_and(|max| next > max) {
            return Err(cost_budget_exceeded());
        }
        ledger.reserved_cost_micros += amount_micros;
        Ok(CostReservation {
            amount_micros,
            currency: currency.to_owned(),
        })
    }

    pub fn settle_cost(
        &self,
        reservation: CostReservation,
        consumed_micros: u128,
    ) -> Result<(), HostError> {
        let mut ledger = self.state.lock()?;
        validate_currency(&ledger, &self.limits, &reservation.currency)?;
        if ledger.reserved_cost_micros < reservation.amount_micros {
            return Err(corrupt_budget_state(
                "cost reservation exceeds the run-owned reserved cost count",
            ));
        }
        ledger.reserved_cost_micros -= reservation.amount_micros;
        ledger.consumed_cost_micros = ledger
            .consumed_cost_micros
            .checked_add(consumed_micros)
            .ok_or_else(cost_budget_exceeded)?;
        let local_max = self.limits.cost.as_ref().map(|cost| cost.max_micros);
        let max = minimum_limit(ledger.max_cost_micros, local_max);
        let committed_and_reserved = ledger
            .consumed_cost_micros
            .checked_add(ledger.reserved_cost_micros)
            .ok_or_else(cost_budget_exceeded)?;
        if max.is_some_and(|max| committed_and_reserved > max) {
            return Err(cost_budget_exceeded());
        }
        Ok(())
    }

    pub fn release_cost(&self, reservation: CostReservation) -> Result<(), HostError> {
        let mut ledger = self.state.lock()?;
        validate_currency(&ledger, &self.limits, &reservation.currency)?;
        if ledger.reserved_cost_micros < reservation.amount_micros {
            return Err(corrupt_budget_state(
                "cost reservation exceeds the run-owned reserved cost count",
            ));
        }
        ledger.reserved_cost_micros -= reservation.amount_micros;
        Ok(())
    }

    pub fn settle_usage(
        &self,
        token_reservation: TokenReservation,
        consumed_tokens: u64,
        cost: Option<(CostReservation, u128)>,
    ) -> Result<(), HostError> {
        let mut ledger = self.state.lock()?;
        if ledger.reserved_tokens < token_reservation.amount {
            return Err(corrupt_budget_state(
                "token reservation exceeds the run-owned reserved token count",
            ));
        }
        let next_tokens = ledger
            .consumed_tokens
            .checked_add(consumed_tokens)
            .ok_or_else(token_budget_exceeded)?;
        let reserved_tokens = ledger.reserved_tokens - token_reservation.amount;
        let token_max = minimum_limit(
            ledger.max_tokens,
            self.limits.tokens.map(|tokens| tokens.max_tokens),
        );
        if token_max.is_some_and(|max| {
            next_tokens
                .checked_add(reserved_tokens)
                .is_none_or(|total| total > max)
        }) {
            return Err(token_budget_exceeded());
        }

        let mut next_cost = ledger.consumed_cost_micros;
        let mut reserved_cost = ledger.reserved_cost_micros;
        if let Some((reservation, consumed_micros)) = &cost {
            validate_currency(&ledger, &self.limits, &reservation.currency)?;
            if reserved_cost < reservation.amount_micros {
                return Err(corrupt_budget_state(
                    "cost reservation exceeds the run-owned reserved cost count",
                ));
            }
            reserved_cost -= reservation.amount_micros;
            next_cost = next_cost
                .checked_add(*consumed_micros)
                .ok_or_else(cost_budget_exceeded)?;
            let cost_max = minimum_limit(
                ledger.max_cost_micros,
                self.limits.cost.as_ref().map(|budget| budget.max_micros),
            );
            if cost_max.is_some_and(|max| {
                next_cost
                    .checked_add(reserved_cost)
                    .is_none_or(|total| total > max)
            }) {
                return Err(cost_budget_exceeded());
            }
        }

        ledger.reserved_tokens = reserved_tokens;
        ledger.consumed_tokens = next_tokens;
        ledger.reserved_cost_micros = reserved_cost;
        ledger.consumed_cost_micros = next_cost;
        Ok(())
    }

    pub fn release_usage(
        &self,
        token_reservation: TokenReservation,
        cost_reservation: Option<CostReservation>,
    ) -> Result<(), HostError> {
        let mut ledger = self.state.lock()?;
        if ledger.reserved_tokens < token_reservation.amount {
            return Err(corrupt_budget_state(
                "token reservation exceeds the run-owned reserved token count",
            ));
        }
        if let Some(reservation) = &cost_reservation {
            validate_currency(&ledger, &self.limits, &reservation.currency)?;
            if ledger.reserved_cost_micros < reservation.amount_micros {
                return Err(corrupt_budget_state(
                    "cost reservation exceeds the run-owned reserved cost count",
                ));
            }
        }
        ledger.reserved_tokens -= token_reservation.amount;
        if let Some(reservation) = cost_reservation {
            ledger.reserved_cost_micros -= reservation.amount_micros;
        }
        Ok(())
    }
}

impl Default for ExecutionBudget {
    fn default() -> Self {
        Self::start(Budget::default())
    }
}

impl fmt::Debug for ExecutionBudget {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ExecutionBudget")
            .field("limits", &self.limits)
            .field("snapshot", &self.snapshot().ok())
            .finish()
    }
}

impl PartialEq for ExecutionBudget {
    fn eq(&self, other: &Self) -> bool {
        self.limits == other.limits && self.snapshot().ok() == other.snapshot().ok()
    }
}

impl Eq for ExecutionBudget {}

impl ExecutionBudgetState {
    fn start(limits: &Budget) -> Self {
        let (deadline, deadline_unix_millis) = deadline_from_limit(limits);
        Self {
            inner: Arc::new(Mutex::new(BudgetLedger {
                deadline,
                deadline_unix_millis,
                max_tokens: limits.tokens.map(|tokens| tokens.max_tokens),
                max_cost_micros: limits.cost.as_ref().map(|cost| cost.max_micros),
                cost_currency: limits.cost.as_ref().map(|cost| cost.currency.clone()),
                reserved_tokens: 0,
                consumed_tokens: 0,
                reserved_cost_micros: 0,
                consumed_cost_micros: 0,
            })),
        }
    }

    fn restore(limits: &Budget, snapshot: ExecutionBudgetSnapshot) -> Result<Self, HostError> {
        if snapshot
            .consumed_tokens
            .checked_add(snapshot.reserved_tokens)
            .is_none()
            || snapshot
                .consumed_cost_micros
                .checked_add(snapshot.reserved_cost_micros)
                .is_none()
        {
            return Err(corrupt_budget_state(
                "execution budget snapshot counters overflow",
            ));
        }
        let deadline = deadline_from_snapshot(snapshot.deadline_unix_millis);
        Ok(Self {
            inner: Arc::new(Mutex::new(BudgetLedger {
                deadline,
                deadline_unix_millis: snapshot.deadline_unix_millis,
                max_tokens: limits.tokens.map(|tokens| tokens.max_tokens),
                max_cost_micros: limits.cost.as_ref().map(|cost| cost.max_micros),
                cost_currency: limits.cost.as_ref().map(|cost| cost.currency.clone()),
                reserved_tokens: snapshot.reserved_tokens,
                consumed_tokens: snapshot.consumed_tokens,
                reserved_cost_micros: snapshot.reserved_cost_micros,
                consumed_cost_micros: snapshot.consumed_cost_micros,
            })),
        })
    }

    fn lock(&self) -> Result<MutexGuard<'_, BudgetLedger>, HostError> {
        self.inner.lock().map_err(|_| {
            HostError::new(
                HostErrorCode::ProviderUnavailable,
                "execution budget state lock is poisoned",
            )
        })
    }

    fn snapshot(&self) -> Result<ExecutionBudgetSnapshot, HostError> {
        let ledger = self.lock()?;
        Ok(ExecutionBudgetSnapshot {
            deadline_unix_millis: ledger.deadline_unix_millis,
            reserved_tokens: ledger.reserved_tokens,
            consumed_tokens: ledger.consumed_tokens,
            reserved_cost_micros: ledger.reserved_cost_micros,
            consumed_cost_micros: ledger.consumed_cost_micros,
        })
    }

    fn check_time(&self) -> Result<(), HostError> {
        let ledger = self.lock()?;
        match ledger.deadline {
            Deadline::Unlimited => Ok(()),
            Deadline::At(deadline) if deadline > Instant::now() => Ok(()),
            Deadline::At(_) => Err(time_budget_exceeded()),
            Deadline::Invalid => Err(corrupt_budget_state(
                "time budget exceeds the runtime clock range",
            )),
        }
    }

    fn deadline(&self) -> Result<Option<tokio::time::Instant>, HostError> {
        let ledger = self.lock()?;
        match ledger.deadline {
            Deadline::Unlimited => Ok(None),
            Deadline::At(deadline) => Ok(Some(tokio::time::Instant::from_std(deadline))),
            Deadline::Invalid => Err(corrupt_budget_state(
                "time budget exceeds the runtime clock range",
            )),
        }
    }
}

fn deadline_from_limit(limits: &Budget) -> (Deadline, Option<u128>) {
    let Some(time) = limits.time else {
        return (Deadline::Unlimited, None);
    };
    let now = Instant::now();
    let duration = Duration::from_millis(time.max_millis);
    let deadline = now
        .checked_add(duration)
        .map_or(Deadline::Invalid, Deadline::At);
    let unix_millis = system_time_millis().map(|now| now + u128::from(time.max_millis));
    (deadline, unix_millis)
}

fn deadline_from_snapshot(deadline_unix_millis: Option<u128>) -> Deadline {
    let Some(deadline_unix_millis) = deadline_unix_millis else {
        return Deadline::Unlimited;
    };
    let Some(now_unix_millis) = system_time_millis() else {
        return Deadline::Invalid;
    };
    if deadline_unix_millis <= now_unix_millis {
        return Deadline::At(Instant::now());
    }
    let remaining = deadline_unix_millis - now_unix_millis;
    let Ok(remaining) = u64::try_from(remaining) else {
        return Deadline::Invalid;
    };
    Instant::now()
        .checked_add(Duration::from_millis(remaining))
        .map_or(Deadline::Invalid, Deadline::At)
}

fn system_time_millis() -> Option<u128> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis())
}

fn minimum_limit<T: Ord + Copy>(global: Option<T>, local: Option<T>) -> Option<T> {
    match (global, local) {
        (Some(global), Some(local)) => Some(global.min(local)),
        (Some(global), None) => Some(global),
        (None, Some(local)) => Some(local),
        (None, None) => None,
    }
}

fn validate_currency(
    ledger: &BudgetLedger,
    limits: &Budget,
    currency: &str,
) -> Result<(), HostError> {
    for expected in [
        ledger.cost_currency.as_deref(),
        limits.cost.as_ref().map(|cost| cost.currency.as_str()),
    ]
    .into_iter()
    .flatten()
    {
        if expected != currency {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "cost reservation currency does not match execution budget",
            )
            .with_detail("expected", expected.to_owned())
            .with_detail("actual", currency.to_owned()));
        }
    }
    Ok(())
}

fn time_budget_exceeded() -> HostError {
    HostError::new(
        HostErrorCode::BudgetExceeded,
        "run-owned execution time budget is exhausted",
    )
}

fn token_budget_exceeded() -> HostError {
    HostError::new(
        HostErrorCode::BudgetExceeded,
        "run-owned execution token budget is exhausted",
    )
}

fn cost_budget_exceeded() -> HostError {
    HostError::new(
        HostErrorCode::BudgetExceeded,
        "run-owned execution cost budget is exhausted",
    )
}

fn corrupt_budget_state(message: impl Into<String>) -> HostError {
    HostError::new(HostErrorCode::ProviderUnavailable, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CostBudget, TimeBudget, TokenBudget};

    #[test]
    fn clones_share_token_reservations_and_consumption() {
        let budget = ExecutionBudget::start(Budget {
            tokens: Some(TokenBudget { max_tokens: 10 }),
            ..Budget::default()
        });
        let clone = budget.clone();
        let first = budget.reserve_tokens(6).expect("first reservation");
        assert_eq!(
            clone
                .reserve_tokens(5)
                .expect_err("clone must observe the shared reservation")
                .code,
            HostErrorCode::BudgetExceeded
        );
        budget
            .settle_tokens(first, 4)
            .expect("reservation should settle");
        let second = clone.reserve_tokens(6).expect("remaining token budget");
        clone
            .settle_tokens(second, 6)
            .expect("second reservation should settle");
        assert_eq!(budget.snapshot().expect("snapshot").consumed_tokens, 10);
    }

    #[test]
    fn snapshot_restores_absolute_deadline_and_counters() {
        let budget = ExecutionBudget::start(Budget {
            tokens: Some(TokenBudget { max_tokens: 10 }),
            time: Some(TimeBudget { max_millis: 10_000 }),
            cost: Some(CostBudget {
                max_micros: 100,
                currency: "USD".to_owned(),
            }),
        });
        let tokens = budget.reserve_tokens(3).expect("token reservation");
        budget.settle_tokens(tokens, 2).expect("token settlement");
        let cost = budget.reserve_cost(30, "USD").expect("cost reservation");
        budget.settle_cost(cost, 20).expect("cost settlement");
        let snapshot = budget.snapshot().expect("budget snapshot");
        let restored = ExecutionBudget::restore(budget.limits().clone(), snapshot.clone())
            .expect("restore budget");

        assert_eq!(restored.snapshot().expect("restored snapshot"), snapshot);
        assert!(restored.deadline().expect("restored deadline").is_some());
    }

    #[test]
    fn scoped_views_share_one_absolute_deadline() {
        let budget = ExecutionBudget::start(Budget {
            time: Some(TimeBudget { max_millis: 10_000 }),
            ..Budget::default()
        });
        let scoped = budget.with_limits(Budget {
            time: Some(TimeBudget { max_millis: 50_000 }),
            ..Budget::default()
        });

        assert_eq!(
            budget.deadline().expect("run deadline"),
            scoped.deadline().expect("scoped deadline"),
            "a scoped request must not restart or extend the run-owned deadline"
        );
    }

    #[test]
    fn token_and_cost_usage_settle_atomically_across_clones() {
        let budget = ExecutionBudget::start(Budget {
            tokens: Some(TokenBudget { max_tokens: 10 }),
            cost: Some(CostBudget {
                max_micros: 100,
                currency: "USD".to_owned(),
            }),
            ..Budget::default()
        });
        let clone = budget.clone();
        let tokens = budget.reserve_tokens(10).expect("token reservation");
        let cost = budget.reserve_cost(100, "USD").expect("cost reservation");
        budget
            .settle_usage(tokens, 4, Some((cost, 30)))
            .expect("combined usage settlement");

        assert_eq!(clone.remaining_tokens().expect("remaining tokens"), Some(6));
        assert_eq!(
            clone.remaining_cost().expect("remaining cost"),
            Some(CostBudget {
                max_micros: 70,
                currency: "USD".to_owned(),
            })
        );
    }
}
