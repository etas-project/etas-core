module std.crypto;

// Public declarations migrated from the legacy API inventory.
// Maintained as Etas source; native bindings retain existing execution during migration.

// Secret-backed crypto error type.
public type CryptoError;

// Nominal cryptographic digest backed by bytes.
public type Digest = bytes;

// Compare byte arrays without data-dependent early exit.
@std.binding("std.crypto.constant_time_eq")
public flow constant_time_eq(arg0: bytes, arg1: bytes) -> bool ![] {}

// Compute HMAC-SHA256 using a host-mediated opaque secret.
@std.binding("std.crypto.hmac_sha256")
public flow hmac_sha256<K>(arg0: std.secret.SecretValue<K>, arg1: bytes) -> std.crypto.Digest ![Error<std.crypto.CryptoError>] {}

// Compute SHA-256 over public bytes.
@std.binding("std.crypto.sha256")
public flow sha256(arg0: bytes) -> std.crypto.Digest ![] {}
