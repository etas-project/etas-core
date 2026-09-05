use std::time::Duration;

use crate::{HostError, HostErrorCode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransportTimeoutPolicy {
    connect_timeout: Duration,
    request_deadline: Duration,
}

impl TransportTimeoutPolicy {
    pub const MIN_MILLIS: u64 = 100;
    pub const DEFAULT_CONNECT_TIMEOUT_MILLIS: u64 = 2_000;
    pub const DEFAULT_REQUEST_DEADLINE_MILLIS: u64 = 30_000;
    pub const MAX_CONNECT_TIMEOUT_MILLIS: u64 = 60_000;
    pub const MAX_REQUEST_DEADLINE_MILLIS: u64 = 3_600_000;

    pub fn try_from_millis(
        connect_timeout_millis: u64,
        request_deadline_millis: u64,
    ) -> Result<Self, HostError> {
        validate_millis(
            "connect timeout",
            connect_timeout_millis,
            Self::MAX_CONNECT_TIMEOUT_MILLIS,
        )?;
        validate_millis(
            "request deadline",
            request_deadline_millis,
            Self::MAX_REQUEST_DEADLINE_MILLIS,
        )?;
        if connect_timeout_millis > request_deadline_millis {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "transport connect timeout cannot exceed request deadline",
            )
            .with_detail("connect_timeout_ms", connect_timeout_millis.to_string())
            .with_detail("request_deadline_ms", request_deadline_millis.to_string()));
        }
        Ok(Self {
            connect_timeout: Duration::from_millis(connect_timeout_millis),
            request_deadline: Duration::from_millis(request_deadline_millis),
        })
    }

    pub fn connect_timeout(self) -> Duration {
        self.connect_timeout
    }

    pub fn request_deadline(self) -> Duration {
        self.request_deadline
    }
}

impl Default for TransportTimeoutPolicy {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_millis(Self::DEFAULT_CONNECT_TIMEOUT_MILLIS),
            request_deadline: Duration::from_millis(Self::DEFAULT_REQUEST_DEADLINE_MILLIS),
        }
    }
}

fn validate_millis(name: &'static str, value: u64, maximum: u64) -> Result<(), HostError> {
    if !(TransportTimeoutPolicy::MIN_MILLIS..=maximum).contains(&value) {
        return Err(HostError::new(
            HostErrorCode::InvalidRequest,
            format!(
                "transport {name} must be between {} and {maximum} milliseconds",
                TransportTimeoutPolicy::MIN_MILLIS
            ),
        )
        .with_detail("value_ms", value.to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeout_policy_enforces_bounds_and_ordering() {
        let policy =
            TransportTimeoutPolicy::try_from_millis(2_000, 30_000).expect("bounded timeout policy");
        assert_eq!(policy.connect_timeout(), Duration::from_secs(2));
        assert_eq!(policy.request_deadline(), Duration::from_secs(30));

        for (connect, request) in [
            (99, 30_000),
            (2_000, 99),
            (60_001, 60_001),
            (2_000, 3_600_001),
            (2_001, 2_000),
        ] {
            assert!(
                TransportTimeoutPolicy::try_from_millis(connect, request).is_err(),
                "invalid timeout policy ({connect}, {request}) must be rejected"
            );
        }
    }
}
