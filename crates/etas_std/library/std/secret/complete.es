module std.secret;

// Secret substrate support type.
public type SecretError = { code: string, message: string };

// Secret substrate support type.
public type SecretKey<K> = { name: string };

// Secret substrate support type.
// A secret is a live, redaction-safe capability rather than readable bytes.
type SecretHandle;
public type SecretValue<K> = { private handle: SecretHandle };

// Read a redaction-safe secret value from the host secret provider.

public flow read<K>(key: SecretKey<K>) -> SecretValue<K> ![Error<SecretError>] {
    return perform std.effects.Secret.read<K>(key);
}
