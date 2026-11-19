module std.tls;

public import std.net.tcp.TcpStream;

// TLS substrate support type.
public type Host = { host: string };

// TLS substrate support type.

// TLS substrate support type.
public type TlsConfig = {  };

// TLS substrate support type.
public type TlsError = { code: string, message: string };

// TLS substrate support type.
type TlsConnection;
public type TlsStream = { private connection: TlsConnection };

// Open a host-mediated TLS client session over a TCP stream.

public flow connect(stream: std.net.tcp.TcpStream, host: Host, config: TlsConfig) -> TlsStream ![Error<TlsError>] {
    return perform std.effects.Tls.handshake(stream, host, config);
}
impl std.tls.TlsStream ~ std.stream.ByteStream {}
