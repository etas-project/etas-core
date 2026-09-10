use super::{SessionMaintenanceOperation, SessionMaintenanceRequest};
use crate::{HostTraceFieldSensitivity, HostTracePayload, HostTraceRequest, HostValue};

impl HostTraceRequest for SessionMaintenanceRequest {
    fn trace_payload(&self) -> HostTracePayload {
        let (action, session, operation, selection) = match &self.operation {
            SessionMaintenanceOperation::Retain(intent) => (
                "Memory.write",
                &intent.session,
                &intent.operation,
                Some(HostValue::Record(vec![
                    (
                        "fence".into(),
                        HostValue::String(intent.fence.as_token().to_owned()),
                    ),
                    (
                        "after_ordinal".into(),
                        HostValue::Int(i128::from(intent.after_ordinal)),
                    ),
                    (
                        "scan_limit".into(),
                        HostValue::UInt(u128::from(intent.scan_limit)),
                    ),
                ])),
            ),
            SessionMaintenanceOperation::Reconcile { session, operation } => {
                ("Memory.read", session, operation, None)
            }
        };
        let mut payload = HostTracePayload::new("session_maintenance", action)
            .with_field(
                "session",
                HostValue::String(session.id.clone()),
                HostTraceFieldSensitivity::Sensitive,
            )
            .with_field(
                "operation",
                HostValue::Record(vec![
                    (
                        "key".into(),
                        HostValue::String(operation.key.as_str().into()),
                    ),
                    (
                        "fingerprint".into(),
                        HostValue::String(operation.request_fingerprint.clone()),
                    ),
                ]),
                HostTraceFieldSensitivity::Sensitive,
            );
        if let Some(selection) = selection {
            payload =
                payload.with_field("retention", selection, HostTraceFieldSensitivity::Sensitive);
        }
        payload
    }
}
