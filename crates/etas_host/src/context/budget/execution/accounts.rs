use super::*;

/// A view of live budget accounts. Conjunction shares these accounts; it never
/// restores their counters or restarts their deadlines.
#[derive(Clone)]
pub struct ExecutionBudget {
    limits: Budget,
    accounts: Vec<BudgetAccount>,
}

#[derive(Clone)]
pub struct ExecutionBudgetState {
    limits: Budget,
    accounts: Vec<BudgetAccount>,
}

impl ExecutionBudget {
    pub fn start(limits: Budget) -> Self {
        Self::start_with_clock(limits, Arc::new(SystemMonotonicClock))
    }

    pub fn start_with_clock(limits: Budget, clock: Arc<dyn MonotonicClock>) -> Self {
        Self {
            accounts: vec![BudgetAccount {
                state: BudgetLedgerState::start(&limits, clock),
                limits: limits.clone(),
            }],
            limits,
        }
    }

    pub fn restore(limits: Budget, snapshot: ExecutionBudgetSnapshot) -> Result<Self, HostError> {
        Self::restore_with_clock(limits, snapshot, Arc::new(SystemMonotonicClock))
    }

    pub fn restore_with_clock(
        limits: Budget,
        snapshot: ExecutionBudgetSnapshot,
        clock: Arc<dyn MonotonicClock>,
    ) -> Result<Self, HostError> {
        Ok(Self {
            accounts: vec![BudgetAccount {
                state: BudgetLedgerState::restore(&limits, snapshot, clock)?,
                limits: limits.clone(),
            }],
            limits,
        })
    }

    pub fn limits(&self) -> &Budget {
        &self.limits
    }

    pub fn with_limits(&self, restriction: Budget) -> Result<Self, HostError> {
        let mut result = self.clone();
        result.limits = stricter_budget_limits(&self.limits, &restriction)?;
        for account in &mut result.accounts {
            account.limits = stricter_budget_limits(&account.limits, &restriction)?;
        }
        Ok(result)
    }

    /// Every operation debits every distinct caller/owner account once. Limits
    /// remain attached to their own counters, rather than applying the smallest
    /// limit to another account's earlier consumption.
    pub fn conjoin(&self, other: &Self) -> Result<Self, HostError> {
        let mut result = self.clone();
        result.limits = stricter_budget_limits(&self.limits, &other.limits)?;
        for account in &other.accounts {
            match result
                .accounts
                .iter_mut()
                .find(|existing| Arc::ptr_eq(&existing.state.inner, &account.state.inner))
            {
                Some(existing) => {
                    existing.limits = stricter_budget_limits(&existing.limits, &account.limits)?;
                }
                None => result.accounts.push(account.clone()),
            }
        }
        Ok(result)
    }

    pub fn replace_limits(&mut self, limits: Budget) {
        *self = Self::start(limits);
    }

    pub fn state(&self) -> ExecutionBudgetState {
        ExecutionBudgetState {
            limits: self.limits.clone(),
            accounts: self.accounts.clone(),
        }
    }

    pub fn rebind_state(&mut self, state: &ExecutionBudgetState) -> Result<(), HostError> {
        *self = Self {
            limits: state.limits.clone(),
            accounts: state.accounts.clone(),
        }
        .with_limits(self.limits.clone())?;
        Ok(())
    }

    pub fn snapshot(&self) -> Result<ExecutionBudgetSnapshot, HostError> {
        if self.accounts.len() != 1 {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "a single-account checkpoint cannot represent multiple live budget accounts",
            ));
        }
        self.accounts[0].state.snapshot()
    }

    /// Observe accounts separately, beginning with this view's original caller
    /// account. Summing them would count a single request's usage multiple times.
    pub fn account_snapshots(&self) -> Result<Vec<ExecutionBudgetSnapshot>, HostError> {
        self.accounts
            .iter()
            .map(|account| account.state.snapshot())
            .collect()
    }

    pub fn resume_under(&self, invocation: &Self) -> Result<Self, HostError> {
        let checkpoint = self.snapshot()?;
        let invocation_snapshot = invocation.snapshot()?;
        let limits = stricter_budget_limits(self.limits(), invocation.limits())?;
        Self::restore(
            limits,
            ExecutionBudgetSnapshot {
                deadline_unix_millis: minimum_limit(
                    checkpoint.deadline_unix_millis,
                    invocation_snapshot.deadline_unix_millis,
                ),
                reserved_tokens: checkpoint
                    .reserved_tokens
                    .checked_add(invocation_snapshot.reserved_tokens)
                    .ok_or_else(token_budget_exceeded)?,
                consumed_tokens: checkpoint
                    .consumed_tokens
                    .checked_add(invocation_snapshot.consumed_tokens)
                    .ok_or_else(token_budget_exceeded)?,
                reserved_cost_micros: checkpoint
                    .reserved_cost_micros
                    .checked_add(invocation_snapshot.reserved_cost_micros)
                    .ok_or_else(cost_budget_exceeded)?,
                consumed_cost_micros: checkpoint
                    .consumed_cost_micros
                    .checked_add(invocation_snapshot.consumed_cost_micros)
                    .ok_or_else(cost_budget_exceeded)?,
            },
        )
    }

    pub fn check_time(&self) -> Result<(), HostError> {
        for account in &self.accounts {
            account.state.check_time()?;
        }
        Ok(())
    }

    pub fn deadline(&self) -> Result<Option<tokio::time::Instant>, HostError> {
        let mut deadline = None;
        for account in &self.accounts {
            deadline = minimum_limit(deadline, account.state.deadline()?);
        }
        Ok(deadline)
    }

    pub fn reserve_tokens(&self, amount: u64) -> Result<TokenReservation, HostError> {
        self.transaction(|account, ledger| account.reserve_tokens(ledger, amount))
    }

    pub fn remaining_tokens(&self) -> Result<Option<u64>, HostError> {
        let mut remaining = None;
        for account in &self.accounts {
            remaining = minimum_limit(
                remaining,
                account.remaining_tokens(&mut *account.state.lock()?)?,
            );
        }
        Ok(remaining)
    }

    pub fn remaining_cost(&self) -> Result<Option<CostBudget>, HostError> {
        let mut remaining: Option<CostBudget> = None;
        for account in &self.accounts {
            if let Some(cost) = account.remaining_cost(&mut *account.state.lock()?)? {
                remaining = Some(match remaining {
                    Some(previous) => CostBudget {
                        max_micros: previous.max_micros.min(cost.max_micros),
                        currency: cost.currency,
                    },
                    None => cost,
                });
            }
        }
        Ok(remaining)
    }

    pub fn settle_tokens(
        &self,
        reservation: TokenReservation,
        consumed: u64,
    ) -> Result<(), HostError> {
        self.transaction(|account, ledger| account.settle_tokens(ledger, reservation, consumed))
    }

    pub fn release_tokens(&self, reservation: TokenReservation) -> Result<(), HostError> {
        self.transaction(|account, ledger| account.release_tokens(ledger, reservation))
    }

    pub fn reserve_cost(
        &self,
        amount_micros: u128,
        currency: &str,
    ) -> Result<CostReservation, HostError> {
        self.transaction(|account, ledger| account.reserve_cost(ledger, amount_micros, currency))
    }

    pub fn settle_cost(
        &self,
        reservation: CostReservation,
        consumed_micros: u128,
    ) -> Result<(), HostError> {
        self.transaction(|account, ledger| {
            account.settle_cost(ledger, reservation.clone(), consumed_micros)
        })
    }

    pub fn release_cost(&self, reservation: CostReservation) -> Result<(), HostError> {
        self.transaction(|account, ledger| account.release_cost(ledger, reservation.clone()))
    }

    pub fn settle_usage(
        &self,
        tokens: TokenReservation,
        consumed_tokens: u64,
        cost: Option<(CostReservation, u128)>,
    ) -> Result<(), HostError> {
        self.transaction(|account, ledger| {
            account.settle_usage(ledger, tokens, consumed_tokens, cost.clone())
        })
    }

    pub fn release_usage(
        &self,
        tokens: TokenReservation,
        cost: Option<CostReservation>,
    ) -> Result<(), HostError> {
        self.transaction(|account, ledger| account.release_usage(ledger, tokens, cost.clone()))
    }

    fn transaction<T>(
        &self,
        update: impl Fn(&BudgetAccount, &mut BudgetLedger) -> Result<T, HostError>,
    ) -> Result<T, HostError> {
        // Lock order is independent of which participant originated this view.
        let mut accounts = self.accounts.iter().collect::<Vec<_>>();
        accounts.sort_by_key(|account| Arc::as_ptr(&account.state.inner) as usize);
        let mut locked = accounts
            .iter()
            .map(|account| account.state.lock())
            .collect::<Result<Vec<_>, _>>()?;
        let mut candidates = locked
            .iter()
            .map(|ledger| (**ledger).clone())
            .collect::<Vec<_>>();
        let mut result = None;
        for (account, ledger) in accounts.iter().zip(&mut candidates) {
            result = Some(update(account, ledger)?);
        }
        for (ledger, candidate) in locked.iter_mut().zip(candidates) {
            **ledger = candidate;
        }
        Ok(result.expect("execution budgets own at least one account"))
    }
}

impl Default for ExecutionBudget {
    fn default() -> Self {
        Self::start(Budget::default())
    }
}

impl fmt::Debug for ExecutionBudget {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let snapshots = self
            .accounts
            .iter()
            .map(|account| account.state.snapshot())
            .collect::<Vec<_>>();
        formatter
            .debug_struct("ExecutionBudget")
            .field("limits", &self.limits)
            .field("accounts", &snapshots)
            .finish()
    }
}

impl PartialEq for ExecutionBudget {
    fn eq(&self, other: &Self) -> bool {
        self.limits == other.limits
            && self.accounts.len() == other.accounts.len()
            && self
                .accounts
                .iter()
                .zip(&other.accounts)
                .all(|(left, right)| {
                    left.limits == right.limits
                        && left.state.snapshot().ok() == right.state.snapshot().ok()
                })
    }
}
impl Eq for ExecutionBudget {}
