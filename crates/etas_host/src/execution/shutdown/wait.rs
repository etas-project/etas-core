use super::super::ExecutionScope;
use super::{PendingWork, TerminationReport};
use crate::HostError;

#[derive(Clone, Debug)]
pub enum StopWait {
    Terminated(TerminationReport),
    TimedOut(PendingWork),
}

impl ExecutionScope {
    /// Dropping this waiter neither stops the run nor removes owned work.
    pub async fn join(&self) -> Result<TerminationReport, HostError> {
        let mut changed = self.subscribe();
        loop {
            if let Some(report) = self.termination()? {
                return Ok(report);
            }
            changed.changed().await.map_err(|_| {
                super::super::scope::invalid_state(
                    "execution observation channel closed before termination",
                )
            })?;
        }
    }

    /// The deadline bounds observation only, not operation/supervisor lifetime.
    pub async fn wait_stopped(
        &self,
        deadline: tokio::time::Instant,
    ) -> Result<StopWait, HostError> {
        match tokio::time::timeout_at(deadline, self.join()).await {
            Ok(report) => report.map(StopWait::Terminated),
            Err(_) => {
                // Query under the registry lock to settle a timeout/completion race.
                if let Some(report) = self.termination()? {
                    Ok(StopWait::Terminated(report))
                } else {
                    Ok(StopWait::TimedOut(self.pending()?))
                }
            }
        }
    }
}
