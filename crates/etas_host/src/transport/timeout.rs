use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeoutConfig {
    pub connect: Duration,
    pub read: Duration,
    pub write: Duration,
}

impl TimeoutConfig {
    pub fn local() -> Self {
        Self {
            connect: Duration::from_millis(500),
            read: Duration::from_secs(2),
            write: Duration::from_secs(2),
        }
    }
}
