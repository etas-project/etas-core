module app.tls_echo;

// Arguments: localhost server port, PEM certificate bundle.
flow main(args: Array<string>) -> i32 ![Error<std.net.tcp.NetworkError>, Error<std.tls.TlsError>, Error<std.stream.StreamError>] {
    let host = std.net.tcp.Host.parse("127.0.0.1").unwrap();
    let port = std.net.tcp.Port.new(std.text.parse_u16(args.get(0).unwrap()).unwrap()).unwrap();
    let timeout = Some(std.runtime.time.Duration.from_millis(2000));
    let stream = std.net.tcp.connect(host, port, std.net.tcp.TcpOptions {
        timeout= timeout,
        nodelay= false,
    });
    let secured = std.tls.connect(stream, host, std.tls.TlsConfig {
        timeout= timeout,
        minimum= std.tls.TlsVersion.V13,
        trust= std.tls.TlsTrust.Bundle(args.get(1).unwrap().utf8),
        alpn= ["etas/1"],
    });
    std.stream.write_all(secured, bytes { data= [112, 105, 110, 103] }, timeout);
    assert(std.stream.read_exact(secured, 4, timeout) == bytes { data= [112, 111, 110, 103] });
    std.stream.close(secured);
    return 0;
}
