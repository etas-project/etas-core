module std.agent.session;

import std.memory.StorageError;

// The runtime creates this capability after backend admission.
public type SessionLease;

// The same VM scope used by std.resource owns and joins the lease cleanup.
type LeaseScope = {};
type LeaseScopeReport<R> = { value: R, failures: Array<std.resource.CleanupFailure> };

impl LeaseScope {
    @std.role("std.resource.Scope.enter")
    flow enter<R, effect E>(body: () -> R ![E], release: (SessionLease) -> unit ![Error<StorageError>]) -> LeaseScopeReport<R> ![E, Error<StorageError>];
}

public flow with_lease<R, effect E>(config: SessionConfig, body: (SessionLease) -> R ![E]) -> R ![E, Error<StorageError>] {
    let report = LeaseScope.enter(() => {
        let lease = perform std.effects.Session.acquire(config);
        return body(lease);
    }, current => {
        perform std.effects.Session.release(current);
    });
    if !report.failures.is_empty() {
        perform Error.raise<StorageError>(StorageError {
            code = "lease_cleanup_failed",
            message = "session lease cleanup failed",
            operation = None,
        });
    }
    return report.value;
}

public flow handoff<R, effect E>(lease: SessionLease, next: (SessionLease) -> R ![E]) -> R ![E, Error<StorageError>] {
    let successor = perform std.effects.Session.handoff(lease);
    return next(successor);
}
