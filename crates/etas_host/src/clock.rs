//! Wall-clock observations and asynchronous monotonic timers.
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::{
    ActionInstance, AuthorityContext, ExecutionBudget, HostError, HostErrorCode, HostRequestId,
    HostTraceFieldSensitivity, HostTracePayload, HostTraceRequest, HostValue, TraceContext,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockOperation {
    Now,
    Monotonic { run: crate::replay::ReplayKeyId },
    Sleep { millis: u128 },
}

impl ClockOperation {
    pub fn action(&self) -> &'static str {
        match self {
            Self::Now => "now",
            Self::Monotonic { .. } => "monotonic",
            Self::Sleep { .. } => "sleep",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClockRequest {
    pub id: HostRequestId,
    pub operation: ClockOperation,
    pub authority: AuthorityContext,
    pub trace: TraceContext,
    pub budget: ExecutionBudget,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClockObservation {
    Time { unix_millis: i128 },
    Instant { domain: String, millis: u128 },
    Woke,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClockResponse {
    pub id: HostRequestId,
    pub result: Result<ClockObservation, HostError>,
}

impl HostTraceRequest for ClockRequest {
    fn trace_payload(&self) -> HostTracePayload {
        let payload = HostTracePayload::new("clock", format!("Clock.{}", self.operation.action()));
        match self.operation {
            ClockOperation::Now | ClockOperation::Monotonic { .. } => payload,
            ClockOperation::Sleep { millis } => payload.with_field(
                "millis",
                HostValue::UInt(millis),
                HostTraceFieldSensitivity::Public,
            ),
        }
    }
}

pub async fn clock(
    request: ClockRequest,
    operation: &crate::execution::OperationContext,
) -> Result<ClockResponse, HostError> {
    request.budget.check_time()?;
    operation.signal().check()?;
    if !request.authority.allows(&ActionInstance::new(
        "Clock",
        request.operation.action(),
        Vec::new(),
    )) {
        return Err(HostError::new(
            HostErrorCode::AuthorityDenied,
            "clock request requires its current Clock action grant",
        ));
    }
    let result = match request.operation {
        ClockOperation::Now => {
            let unix_millis = match SystemTime::now().duration_since(UNIX_EPOCH) {
                Ok(elapsed) => elapsed.as_millis() as i128,
                Err(earlier) => {
                    // Millisecond timestamps before the epoch round towards
                    // the earlier millisecond, as timestamps after it do.
                    let nanos = earlier.duration().as_nanos();
                    -(nanos.div_ceil(1_000_000) as i128)
                }
            };
            Ok(ClockObservation::Time { unix_millis })
        }
        ClockOperation::Monotonic { run } => monotonic(run),
        ClockOperation::Sleep { millis } => {
            let seconds = u64::try_from(millis / 1000).map_err(|_| timer_overflow())?;
            let duration = Duration::new(seconds, ((millis % 1000) * 1_000_000) as u32);
            let wake = tokio::time::Instant::now()
                .checked_add(duration)
                .ok_or_else(timer_overflow)?;
            if millis != 0 {
                tokio::select! {
                    _ = tokio::time::sleep_until(wake) => {},
                    _ = operation.signal().cancelled() => { operation.signal().check()?; }
                }
            }
            operation.signal().check()?;
            request.budget.check_time()?;
            Ok(ClockObservation::Woke)
        }
    };
    Ok(ClockResponse {
        id: request.id,
        result,
    })
}

fn timer_overflow() -> HostError {
    HostError::new(
        HostErrorCode::InvalidRequest,
        "clock timer duration overflows the platform",
    )
    .with_detail("time_error", "Overflow")
}

struct ProcessClock {
    identity: crate::replay::ReplayKeyId,
    epoch: std::time::Instant,
}

fn monotonic(run: crate::replay::ReplayKeyId) -> Result<ClockObservation, HostError> {
    // One epoch per process; a restored observation from a different process
    // keeps its old domain and cannot silently acquire this new epoch.
    static CLOCK: std::sync::Mutex<Option<ProcessClock>> = std::sync::Mutex::new(None);
    let mut slot = CLOCK.lock().expect("process clock mutex poisoned");
    if slot.is_none() {
        let identity = crate::replay::ReplayKey::generate()
            .map_err(|_| {
                HostError::new(
                    HostErrorCode::ProviderUnavailable,
                    "monotonic clock domain unavailable",
                )
            })?
            .id();
        *slot = Some(ProcessClock {
            identity,
            epoch: std::time::Instant::now(),
        });
    }
    let clock = slot.as_ref().unwrap();
    Ok(ClockObservation::Instant {
        domain: format!("{}/{run}", clock.identity),
        millis: clock.epoch.elapsed().as_millis(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[tokio::test(flavor = "current_thread")]
    async fn timer_allows_other_work_on_the_same_thread_and_never_wakes_early() {
        let scope = crate::execution::ExecutionScope::new_owned();
        let trace = TraceContext::root(crate::TraceId(1));
        let registration = scope
            .register(None, Some(HostRequestId(1)), trace.clone())
            .unwrap();
        registration.begin_dispatch().unwrap();
        let mut authority = AuthorityContext::deny_all();
        authority
            .grants
            .push(crate::HostActionGrant::allow("Clock", "sleep"));
        let request = ClockRequest {
            id: HostRequestId(1),
            operation: ClockOperation::Sleep { millis: 20 },
            authority,
            trace,
            budget: ExecutionBudget::default(),
        };
        let finished = Cell::new(false);
        let started = std::time::Instant::now();
        let (response, ()) = tokio::join!(
            async {
                let response = clock(request, registration.context()).await.unwrap();
                finished.set(true);
                response
            },
            async {
                assert!(!finished.get(), "timer blocked its scheduler thread");
            },
        );
        assert_eq!(response.result, Ok(ClockObservation::Woke));
        assert!(started.elapsed() >= Duration::from_millis(20));
        registration
            .complete(crate::execution::ExternalOutcome::Confirmed, vec![])
            .unwrap();
        scope.finish_body(true).unwrap();
        scope.join().await.unwrap();
        let run = crate::replay::ReplayKeyId::from_bytes([1; 32]);
        let ClockObservation::Instant { domain, millis } = monotonic(run).unwrap() else {
            unreachable!()
        };
        let ClockObservation::Instant {
            domain: same,
            millis: later,
        } = monotonic(run).unwrap()
        else {
            unreachable!()
        };
        assert_eq!(domain, same);
        assert!(later >= millis);
        let ClockObservation::Instant { domain: other, .. } =
            monotonic(crate::replay::ReplayKeyId::from_bytes([2; 32])).unwrap()
        else {
            unreachable!()
        };
        assert_ne!(domain, other);
    }
}
