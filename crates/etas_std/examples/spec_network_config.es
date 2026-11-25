module app.network_config;

flow main(args: Array<string>) -> i32 ![] {
    let dns = std.net.tcp.Host.parse("Example.COM.").unwrap();
    assert(dns.to_string() == "example.com.");
    let ip: std.tls.Host = std.net.tcp.Host.parse("127.0.0.1").unwrap();
    assert(ip.to_string() == "127.0.0.1");
    let ipv6 = std.net.tcp.Host.parse("[2001:0DB8:0:0:0:0:0:1]").unwrap();
    assert(ipv6.to_string() == "2001:db8::1");
    let mapped = std.net.tcp.Host.parse("::ffff:192.0.2.1").unwrap();
    assert(mapped.to_string() == "::ffff:192.0.2.1");
    let port = std.net.tcp.Port.new(443).unwrap();
    assert(port.value() == 443);
    let tcp = std.net.tcp.DefaultTcpOptions;
    assert(tcp.nodelay);
    assert(tcp.timeout.is_none());
    let tls = std.tls.DefaultTlsConfig;
    assert(tls.timeout.is_none());
    assert(tls.alpn.length == 0);
    assert(match tls.minimum { std.tls.TlsVersion.V12 => true, std.tls.TlsVersion.V13 => false, });
    assert(match tls.trust { std.tls.TlsTrust.Platform => true, std.tls.TlsTrust.Bundle(_) => false, });
    return 0;
}
