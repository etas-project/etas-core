module std.tls;

public import std.net.tcp.{Host, TcpStream};

public enum TlsVersion { V12, V13, }
public enum TlsTrust { Platform, Bundle(bytes), }

public type TlsConfig = {
    timeout: Option<std.stream.Timeout>,
    minimum: TlsVersion,
    trust: TlsTrust,
    alpn: Array<string>,
};

public let DefaultTlsConfig = TlsConfig {
    timeout= None,
    minimum= TlsVersion.V12,
    trust= TlsTrust.Platform,
    alpn= [],
};

public type TlsError = { code: string, message: string };

type TlsConnection;
public type TlsStream = { private connection: TlsConnection };

public flow connect(stream: TcpStream, host: Host, config: TlsConfig = DefaultTlsConfig) -> TlsStream ![Error<TlsError>] {
    return perform std.effects.Tls.handshake(stream, host, config);
}

impl TlsStream ~ std.stream.ByteStream {}
