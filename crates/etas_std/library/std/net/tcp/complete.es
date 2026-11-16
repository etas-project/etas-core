module std.net.tcp;

// Public declarations migrated from the legacy API inventory.
// Maintained as Etas source; native bindings retain existing execution during migration.

// TCP substrate support type.
public type Host = { host: string };

// TCP substrate support type.
public type NetworkError;

// TCP substrate support type.
public type Port = { port: i32 };

// TCP substrate support type.
public type TcpOptions = {  };

// TCP substrate support type.
public type TcpStream;

// Open a host-mediated TCP connection.
@std.binding("std.net.tcp.connect")
public flow connect(arg0: std.net.tcp.Host, arg1: std.net.tcp.Port, arg2: std.net.tcp.TcpOptions) -> std.net.tcp.TcpStream ![Error<std.net.tcp.NetworkError>] {}

impl std.net.tcp.TcpStream ~ std.stream.ByteStream {}
