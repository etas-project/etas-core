use tokio::io::{AsyncRead, AsyncReadExt};

use crate::{HostError, HostErrorCode};

pub(super) async fn collect_bounded(
    mut reader: impl AsyncRead + Unpin,
    stream: &'static str,
    limit: usize,
) -> Result<Vec<u8>, HostError> {
    let mut output = Vec::with_capacity(limit.min(8192));
    let mut chunk = [0_u8; 8192];
    loop {
        let read = reader.read(&mut chunk).await.map_err(|error| {
            HostError::new(
                HostErrorCode::ProviderUnavailable,
                "failed to read command output",
            )
            .with_detail("stream", stream)
            .with_detail("error", error.to_string())
        })?;
        if read == 0 {
            return Ok(output);
        }
        let next_len = output
            .len()
            .checked_add(read)
            .ok_or_else(|| output_limit_exceeded(stream, limit))?;
        if next_len > limit {
            return Err(output_limit_exceeded(stream, limit));
        }
        output.extend_from_slice(&chunk[..read]);
    }
}

fn output_limit_exceeded(stream: &'static str, limit: usize) -> HostError {
    HostError::new(
        HostErrorCode::BudgetExceeded,
        "command output exceeded the configured capture limit",
    )
    .with_detail("stream", stream)
    .with_detail("limit_bytes", limit.to_string())
}
