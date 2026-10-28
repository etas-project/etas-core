mod measurement;

use std::{num::NonZeroU64, thread::ThreadId, time::Duration};

use crate::{HostError, HostErrorCode};

#[cfg(test)]
mod tests;

/// Cooperative native-worker ceilings. The memory ceiling covers the entire
/// embedding process, including its pre-existing resident memory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompilerResourceLimits {
    max_worker_cpu_time: Duration,
    max_process_peak_resident_bytes: NonZeroU64,
}

impl CompilerResourceLimits {
    pub fn new(
        max_worker_cpu_time: Duration,
        max_process_peak_resident_bytes: NonZeroU64,
    ) -> Result<Self, HostError> {
        if max_worker_cpu_time.is_zero() {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "compiler CPU ceiling must be a nonzero duration",
            ));
        }
        Ok(Self {
            max_worker_cpu_time,
            max_process_peak_resident_bytes,
        })
    }

    pub fn max_worker_cpu_time(self) -> Duration {
        self.max_worker_cpu_time
    }

    pub fn max_process_peak_resident_bytes(self) -> NonZeroU64 {
        self.max_process_peak_resident_bytes
    }

    fn check(self, usage: CompilerResourceUsage) -> Result<(), HostError> {
        if usage.worker_cpu_time >= self.max_worker_cpu_time {
            return Err(exhausted(
                "worker_cpu_time",
                "nanoseconds",
                usage.worker_cpu_time.as_nanos(),
                self.max_worker_cpu_time.as_nanos(),
            ));
        }
        if usage.process_peak_resident_bytes > self.max_process_peak_resident_bytes.get() {
            return Err(exhausted(
                "process_peak_resident_memory",
                "bytes",
                usage.process_peak_resident_bytes.into(),
                self.max_process_peak_resident_bytes.get().into(),
            ));
        }
        Ok(())
    }
}

impl Default for CompilerResourceLimits {
    fn default() -> Self {
        Self {
            max_worker_cpu_time: Duration::from_secs(60),
            max_process_peak_resident_bytes: NonZeroU64::new(2 * 1024 * 1024 * 1024)
                .expect("nonzero compiler process memory ceiling"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompilerResourceUsage {
    pub worker_cpu_time: Duration,
    pub process_peak_resident_bytes: u64,
}

/// Created and sampled on the actual blocking worker, never its async receiver.
pub struct CompilerResourceMeter {
    worker: ThreadId,
    start_cpu_time: Duration,
    limits: CompilerResourceLimits,
}

impl CompilerResourceMeter {
    pub fn start(limits: CompilerResourceLimits) -> Result<Self, HostError> {
        let (start_cpu_time, process_peak_resident_bytes) = measurement::sample()?;
        limits.check(CompilerResourceUsage {
            worker_cpu_time: Duration::ZERO,
            process_peak_resident_bytes,
        })?;
        Ok(Self {
            worker: std::thread::current().id(),
            start_cpu_time,
            limits,
        })
    }

    pub fn check(&self) -> Result<CompilerResourceUsage, HostError> {
        if std::thread::current().id() != self.worker {
            return Err(unavailable(
                "compiler resource meter used outside its worker",
            ));
        }
        let (cpu_time, process_peak_resident_bytes) = measurement::sample()?;
        let usage = CompilerResourceUsage {
            worker_cpu_time: cpu_time
                .checked_sub(self.start_cpu_time)
                .ok_or_else(|| unavailable("compiler worker CPU clock moved backwards"))?,
            process_peak_resident_bytes,
        };
        self.limits.check(usage)?;
        Ok(usage)
    }
}

fn exhausted(resource: &str, unit: &str, used: u128, limit: u128) -> HostError {
    HostError::new(
        HostErrorCode::BudgetExceeded,
        "compiler resource ceiling exceeded",
    )
    .with_detail("resource", resource)
    .with_detail("unit", unit)
    .with_detail("used", used.to_string())
    .with_detail("limit", limit.to_string())
}

fn unavailable(reason: impl Into<String>) -> HostError {
    HostError::new(HostErrorCode::ProviderUnavailable, reason)
}
