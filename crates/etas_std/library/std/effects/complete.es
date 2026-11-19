module std.effects;

// Human approval effect through the runtime approval boundary.
@std.role("std.effects.Approval")
public effect Approval extends Human {
    action<T> request(request: std.runtime.approval.ApprovalRequest<T>) -> std.runtime.approval.ApprovalDecision;
}

// Browser protocol/session substrate action boundary.
@std.role("std.effects.Browser")
public effect Browser extends Network {
    @default_handler
    action attach<Capability0: std.browser.protocol.BrowserProfile>(arg0: std.browser.protocol.BrowserProfile) -> std.browser.protocol.BrowserSession ![Error<std.browser.protocol.BrowserError>];
    @default_handler
    action create<Capability0: std.browser.protocol.BrowserProfile>(profile: std.browser.protocol.BrowserProfile) -> std.browser.protocol.BrowserSession ![Error<std.browser.protocol.BrowserError>];
    @default_handler
    action close<Capability0: std.browser.protocol.BrowserSession>(arg0: std.browser.protocol.BrowserSession) -> unit ![Error<std.browser.protocol.BrowserError>];
    @default_handler
    action recv<Capability0: std.browser.protocol.BrowserSession>(arg0: std.browser.protocol.BrowserSession) -> std.browser.protocol.BrowserMessage ![Error<std.browser.protocol.BrowserError>];
    @default_handler
    action screenshot<Capability0: std.browser.protocol.BrowserSession>(arg0: std.browser.protocol.BrowserSession) -> std.browser.protocol.BrowserScreenshot ![Error<std.browser.protocol.BrowserError>];
    @default_handler
    action send<Capability0: std.browser.protocol.BrowserSession>(session: std.browser.protocol.BrowserSession, message: std.browser.protocol.BrowserMessage) -> unit ![Error<std.browser.protocol.BrowserError>];
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
    @default_handler
    action<A> check(arg0: std.compiler.AgentDef<A>) -> std.compiler.CheckReport ![Error<std.compiler.CompileError>];
    @default_handler
    action<F> check(arg0: std.compiler.CallableDef<F>) -> std.compiler.CheckReport ![Error<std.compiler.CompileError>];
    @default_handler
    action<A> compile(arg0: std.compiler.AgentDef<A>) -> A ![Error<std.compiler.CompileError>];
    @default_handler
    action<F> compile(arg0: std.compiler.CallableDef<F>) -> F ![Error<std.compiler.CompileError>];
}

// TCP connection substrate action boundary.
@std.role("std.effects.Net")
public effect Net extends Network {
    @default_handler
    action tcp_connect<HostPattern: string, PortPattern: string>(host: std.net.tcp.Host, port: std.net.tcp.Port, options: std.net.tcp.TcpOptions) -> std.net.tcp.TcpStream ![Error<std.net.tcp.NetworkError>];
}

// Bounded byte stream substrate action boundary.
@std.role("std.effects.Stream")
public effect Stream {
    @default_handler
    action close<S ~ std.stream.ByteStream>(arg0: S) -> unit ![Error<std.stream.StreamError>];
    @default_handler
    action flush<S ~ std.stream.ByteStream>(arg0: S) -> unit ![Error<std.stream.StreamError>];
    @default_handler
    action read<S ~ std.stream.ByteStream>(stream: S, max_bytes: usize, timeout: Option<std.stream.Timeout>) -> std.stream.StreamRead ![Error<std.stream.StreamError>];
    @default_handler
    action write<S ~ std.stream.ByteStream>(stream: S, body: bytes) -> unit ![Error<std.stream.StreamError>];
}

// TLS client session substrate action boundary.
@std.role("std.effects.Tls")
public effect Tls extends Network {
    @default_handler
    action handshake<HostPattern: string>(stream: std.net.tcp.TcpStream, host: std.tls.Host, config: std.tls.TlsConfig) -> std.tls.TlsStream ![Error<std.tls.TlsError>];
}

// Actual session store operations. Default handling retains the precise
// Memory.read/write session place; this declaration grants no broad authority.
public effect Session extends Memory {
    @default_handler
    action resolve(config: std.agent.session.SessionConfig) -> std.agent.session.SessionId ![];
    @default_handler
    action<T> append(message: std.agent.message.Message<T>) -> std.agent.message.Message<T> ![];
    @default_handler
    action read(config: std.agent.session.SessionConfig, cursor: Option<std.agent.session.SessionCursor>, limit: Option<u32>) -> std.agent.session.SessionHistoryPage ![Error<std.memory.StorageError>];
    @default_handler
    action publish(config: std.agent.session.SessionConfig, fence: std.agent.session.SessionHistoryFence, content: std.agent.session.SessionContextContent, operation: std.memory.StorageOperationRef) -> std.memory.WriteOutcome<std.agent.session.SessionContextReceipt, std.agent.session.SessionContextRejection> ![Error<std.memory.StorageError>];
    @default_handler
    action reconcile(config: std.agent.session.SessionConfig, operation: std.memory.StorageOperationRef) -> std.memory.ReconcileResult<std.agent.session.SessionContextReceipt, std.agent.session.SessionContextRejection> ![Error<std.memory.StorageError>];
}

// One checked native-tool invocation. C retains the current exposed-tool set,
// latent errors/effects and trace facts; transparent protocol data grants none.
public effect Tool {
    @default_handler
    action call<C>(tool: std.agent.ToolRef, args: std.json.JsonValue, call_id: string) -> Result<std.json.JsonValue, std.agent.StepDiagnostic> ![];
}
