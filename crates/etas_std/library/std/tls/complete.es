module std.tls;

// Public declarations migrated from the legacy API inventory.
// Maintained as Etas source; native bindings retain existing execution during migration.

// TLS substrate support type.
public type Host = { host: string };

// TLS substrate support type.
public type TcpStream;

// TLS substrate support type.
public type TlsConfig = {  };

// TLS substrate support type.
public type TlsError;

// TLS substrate support type.
public type TlsStream;

// Open a host-mediated TLS client session over a TCP stream.
@std.binding("std.tls.connect")
public flow connect(arg0: std.net.tcp.TcpStream, arg1: std.tls.Host, arg2: std.tls.TlsConfig) -> std.tls.TlsStream ![Error<std.tls.TlsError>] {}

impl std.tls.TlsStream ~ std.stream.ByteStream {}
