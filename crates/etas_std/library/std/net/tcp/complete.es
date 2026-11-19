module std.net.tcp;

// TCP substrate support type.
public type Host = { host: string };

// TCP substrate support type.
public type NetworkError = { code: string, message: string };

// TCP substrate support type.
public type Port = { port: i32 };

// TCP substrate support type.
public type TcpOptions = {  };

// TCP substrate support type.
// Only the host connection primitive creates a live connection handle.
type TcpConnection;
public type TcpStream = { private connection: TcpConnection };

// Open a host-mediated TCP connection.

public flow connect(host: Host, port: Port, options: TcpOptions) -> TcpStream ![Error<NetworkError>] {
    return perform std.effects.Net.tcp_connect(host, port, options);
}
impl std.net.tcp.TcpStream ~ std.stream.ByteStream {}
