module std.security.trust;

// Public and untrusted payloads require no authority to construct.
public type Public<T> = { value: T };
public type Untrusted<T> = { value: T };

// Evidence is an execution capability; the payload and wrapper are source data.
// The primitive returns these handles. There is no source handle constructor.
type TrustEvidence;
type SecretEvidence;

public type Trusted<T> = {
    private value: T,
    private evidence: TrustEvidence,
};

public type Secret<T> = {
    private value: T,
    private evidence: SecretEvidence,
};

public type Sanitized<T> = {
    value: T,
    private evidence: std.security.declassify.SanitizationEvidence<T>,
};
