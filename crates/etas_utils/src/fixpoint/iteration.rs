#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IterationLimit {
    max_iterations: usize,
}

impl IterationLimit {
    pub const DEFAULT: Self = Self {
        max_iterations: 10_000,
    };

    pub const fn new(max_iterations: usize) -> Self {
        Self { max_iterations }
    }

    pub const fn max_iterations(self) -> usize {
        self.max_iterations
    }
}

impl Default for IterationLimit {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConvergenceStatus {
    Converged,
    IterationLimitReached,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FixpointStats {
    pub iterations: usize,
    pub changes: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FixpointResult<T> {
    pub value: T,
    pub status: ConvergenceStatus,
    pub stats: FixpointStats,
}

impl<T> FixpointResult<T> {
    pub fn converged(&self) -> bool {
        self.status == ConvergenceStatus::Converged
    }
}
