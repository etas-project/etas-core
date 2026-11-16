module std.secret;

// Public declarations migrated from the legacy API inventory.
// Maintained as Etas source; native bindings retain existing execution during migration.

// Secret substrate support type.
public type SecretError;

// Secret substrate support type.
public type SecretKey<K>;

// Secret substrate support type.
public type SecretValue<K>;

// Read a redaction-safe secret value from the host secret provider.
@std.binding("std.secret.read")
public flow read<K>(arg0: std.secret.SecretKey<K>) -> std.secret.SecretValue<K> ![Error<std.secret.SecretError>] {}
