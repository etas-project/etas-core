use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetryPolicy {
    pub attempts: u8,
    pub delay: Duration,
}

impl RetryPolicy {
    pub fn none() -> Self {
        Self {
            attempts: 1,
            delay: Duration::ZERO,
        }
    }
}
