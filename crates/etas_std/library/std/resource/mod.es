module std.resource;

// Cleanup diagnostics are ordinary redacted data, never a resource capability.
public type CleanupFailure = {
    resource_kind: string,
    code: string,
    message: string,
};

public enum ResourceError {
    CleanupFailed(Array<CleanupFailure>),
}

// Scope admission and its returned diagnostics are private VM mechanisms.
type Scope = {};
type ScopeReport<R> = { value: R, failures: Array<CleanupFailure> };

impl Scope {
    @std.role("std.resource.Scope.enter")
    flow enter<R, effect E>(body: () -> R ![E]) -> ScopeReport<R> ![E];
}

public flow scope<R, effect E>(body: () -> R ![E]) -> R ![E, Error<ResourceError>] {
    let report = Scope.enter(body);
    if !report.failures.is_empty() {
        perform Error.raise<ResourceError>(ResourceError.CleanupFailed(report.failures));
    }
    return report.value;
}
