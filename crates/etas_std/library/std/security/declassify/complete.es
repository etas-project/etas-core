module std.security.declassify;

// Only an execution primitive can create evidence/authorization handles.
type SanitizationHandle;
type DeclassificationPermit;

public type SanitizationEvidence<U> = {
    private handle: SanitizationHandle,
};

public type SanitizationStrategy<T, U> = {
    private transform: (T) -> Result<U, SecurityError> ![],
    private evidence: SanitizationEvidence<U>,
};

public type DeclassificationStrategy<T, U> = {
    private transform: (T) -> Result<U, SecurityError> ![],
    private evidence: SanitizationEvidence<U>,
    private permit: DeclassificationPermit,
};

public enum SecurityError {
    Rejected,
    Denied,
    InvalidReason,
    Unavailable,
    StrategyMismatch,
}

public flow sanitize<T, U>(value: Untrusted<T>, strategy: SanitizationStrategy<T, U>)
    -> Sanitized<U> ![Error<SecurityError>] {
    let transformed = (strategy.transform)(value.value);
    return match transformed {
        Ok(output) => Sanitized { value= output, evidence= strategy.evidence },
        Err(error) => perform Error.raise<SecurityError>(error),
    };
}

public flow declassify<T, U>(value: Secret<T>, strategy: DeclassificationStrategy<T, U>, reason: string)
    -> Sanitized<U> ![std.effects.Secret, Error<SecurityError>] {
    if std.text.is_empty(std.text.trim(reason)) {
        perform Error.raise<SecurityError>(SecurityError.InvalidReason);
    }
    return perform std.effects.Secret.declassify(value, strategy, reason);
}
