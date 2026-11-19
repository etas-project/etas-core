module std.agent.session;

import std.memory.{StorageContext, StorageError};


impl SessionHistoryFence {
    // Parse shape and subject only. This does not authenticate the seal or
    // certify freshness; the backend checks both atomically at publication.
    flow subject(self, context: StorageContext) -> string ![Error<StorageError>] {
        if self.opaque.length > context.max_value_bytes {
            perform Error.raise<StorageError>(StorageError { code= "budget_exceeded", message= "session history fence exceeds storage limits" });
        }
        let document = FenceValue.parse(self.opaque, context.max_depth);
        let envelope = document.object(["claims", "seal"], []);
        envelope.get("seal").unwrap().hex(64);
        let claims_value = envelope.get("claims").unwrap();
        let claims = claims_value.object(["format", "selection", "incarnation", "revision", "context_version"], []);
        if claims.get("format").unwrap().unsigned() != 1 { document.invalid(); }
        claims.get("incarnation").unwrap().hex(32);
        claims.get("revision").unwrap().hex(32);
        let version = claims.get("context_version").unwrap().unsigned();
        let selection = claims.get("selection").unwrap().object(["version", "session", "context", "upper", "lower", "after"], ["retained_after"]);
        if selection.get("version").unwrap().unsigned() != 2 { document.invalid(); }
        let session = selection.get("session").unwrap().text();
        let policy = selection.get("context").unwrap().text();
        let upper = selection.get("upper").unwrap().integer();
        let lower = selection.get("lower").unwrap().integer();
        let after = selection.get("after").unwrap().integer();
        if session == "" || policy == "" || upper < -1 || upper >= 9223372036854775807 || lower < 0 || after < -1 || after > upper {
            document.invalid();
        }
        if lower - 1 > upper || after != lower - 1 { document.invalid(); }
        match selection.get("retained_after") {
            None => {},
            Some(FenceValue.Null) => {},
            Some(value) => { let cutoff = value.integer128(); },
        };
        return session;
    }
}
