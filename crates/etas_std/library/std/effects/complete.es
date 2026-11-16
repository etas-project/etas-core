module std.effects;

// Public declarations migrated from the legacy API inventory.
// Maintained as Etas source; native bindings retain existing execution during migration.

// Human approval effect through the runtime approval boundary.
@std.role("std.effects.Approval")
public effect Approval extends Human {
    action request(arg0: std.runtime.approval.ApprovalRequest) -> std.runtime.approval.ApprovalDecision;
}

// Browser protocol/session substrate action boundary.
@std.role("std.effects.Browser")
public effect Browser extends Network {
    action attach<Capability0: std.browser.protocol.BrowserProfile>(arg0: std.browser.protocol.BrowserProfile) -> std.browser.protocol.BrowserSession;
    action close<Capability0: std.browser.protocol.BrowserSession>(arg0: std.browser.protocol.BrowserSession) -> unit;
    action recv<Capability0: std.browser.protocol.BrowserSession>(arg0: std.browser.protocol.BrowserSession) -> std.browser.protocol.BrowserMessage;
    action screenshot<Capability0: std.browser.protocol.BrowserSession>(arg0: std.browser.protocol.BrowserSession) -> std.browser.protocol.BrowserScreenshot;
    action send<Capability0: std.browser.protocol.BrowserSession>(arg0: std.browser.protocol.BrowserSession) -> unit;
}

// Standard clock action boundary.
@std.role("std.effects.Clock")
public effect Clock extends Time {
    action now() -> std.runtime.time.Time;
    action sleep(arg0: std.runtime.time.Duration) -> unit;
}

// Checking and compilation of immutable retained definitions.
@std.role("std.effects.Code")
public effect Code {
    action<A> check(arg0: std.compiler.AgentDef<A>) -> std.compiler.CheckReport;
    action<F> check(arg0: std.compiler.CallableDef<F>) -> std.compiler.CheckReport;
    action<A> compile(arg0: std.compiler.AgentDef<A>) -> A;
    action<F> compile(arg0: std.compiler.CallableDef<F>) -> F;
}

// TCP connection substrate action boundary.
@std.role("std.effects.Net")
public effect Net extends Network {
    action tcp_connect<HostPattern: string, PortPattern: string>(arg0: std.net.tcp.Host, arg1: std.net.tcp.Port) -> std.net.tcp.TcpStream;
}

// Bounded byte stream substrate action boundary.
@std.role("std.effects.Stream")
public effect Stream {
    action close<S ~ std.stream.ByteStream>(arg0: S) -> unit;
    action flush<S ~ std.stream.ByteStream>(arg0: S) -> unit;
    action read<S ~ std.stream.ByteStream>(arg0: S) -> std.stream.StreamRead;
    action write<S ~ std.stream.ByteStream>(arg0: S) -> unit;
}

// TLS client session substrate action boundary.
@std.role("std.effects.Tls")
public effect Tls extends Network {
    action handshake<HostPattern: string>(arg0: std.tls.Host) -> std.tls.TlsStream;
}
