use crate::console::{ConsoleOperation, ConsoleRequest};
use crate::{
    BrowserProtocolOperation, BrowserProtocolRequest, ByteStreamOrigin, ByteStreamRef,
    CommandRequest, FilesystemOperation, FilesystemRequest, HostTraceFieldSensitivity,
    HostTracePayload, HostValue, SecretOperation, SecretRequest, StreamOperation, StreamRequest,
    TcpConnectOperation, TcpConnectRequest, TcpStreamRef, TlsConnectOperation, TlsConnectRequest,
    WorkspacePathRef,
};

use super::{HostTraceRequest, option, record, variant};

impl HostTraceRequest for ConsoleRequest {
    fn trace_payload(&self) -> HostTracePayload {
        let (action, operation) = match &self.operation {
            ConsoleOperation::ReadAllStdin => (
                "Console.stdin_read_all",
                variant("ReadAllStdin", Vec::new()),
            ),
            ConsoleOperation::ReadLineStdin => (
                "Console.stdin_read_line",
                variant("ReadLineStdin", Vec::new()),
            ),
            ConsoleOperation::WriteStdout { text, newline } => (
                "Console.stdout_write",
                variant(
                    "WriteStdout",
                    vec![HostValue::String(text.clone()), HostValue::Bool(*newline)],
                ),
            ),
            ConsoleOperation::WriteStderr { text, newline } => (
                "Console.stderr_write",
                variant(
                    "WriteStderr",
                    vec![HostValue::String(text.clone()), HostValue::Bool(*newline)],
                ),
            ),
        };
        HostTracePayload::new("console", action).with_field(
            "operation",
            operation,
            HostTraceFieldSensitivity::Sensitive,
        )
    }
}

impl HostTraceRequest for FilesystemRequest {
    fn trace_payload(&self) -> HostTracePayload {
        let (action, operation) = match &self.operation {
            FilesystemOperation::Read { path } => {
                ("Fs.read", variant("Read", vec![workspace_path(path)]))
            }
            FilesystemOperation::Write {
                path,
                contents,
                create_dirs,
            } => (
                "Fs.write",
                variant(
                    "Write",
                    vec![
                        workspace_path(path),
                        HostValue::Bytes(contents.clone()),
                        HostValue::Bool(*create_dirs),
                    ],
                ),
            ),
            FilesystemOperation::Delete { path } => (
                "HostFilesystem.delete",
                variant("Delete", vec![workspace_path(path)]),
            ),
            FilesystemOperation::ReadDir { path } => {
                ("Fs.list", variant("ReadDir", vec![workspace_path(path)]))
            }
            FilesystemOperation::Stat { path } => {
                ("Fs.stat", variant("Stat", vec![workspace_path(path)]))
            }
            FilesystemOperation::AtomicReplace { path, contents } => (
                "Fs.atomic_replace",
                variant(
                    "AtomicReplace",
                    vec![workspace_path(path), HostValue::Bytes(contents.clone())],
                ),
            ),
        };
        HostTracePayload::new("filesystem", action).with_field(
            "operation",
            operation,
            HostTraceFieldSensitivity::Sensitive,
        )
    }
}

impl HostTraceRequest for CommandRequest {
    fn trace_payload(&self) -> HostTracePayload {
        HostTracePayload::new("command", "Command.run")
            .with_field(
                "argv",
                HostValue::List(self.argv.iter().cloned().map(HostValue::String).collect()),
                HostTraceFieldSensitivity::Sensitive,
            )
            .with_field(
                "env",
                HostValue::Record(
                    self.env
                        .iter()
                        .map(|(name, value)| (name.clone(), HostValue::String(value.clone())))
                        .collect(),
                ),
                HostTraceFieldSensitivity::Secret,
            )
            .with_field(
                "cwd",
                option(self.cwd.as_ref().map(workspace_path)),
                HostTraceFieldSensitivity::Sensitive,
            )
            .with_field(
                "stdin",
                option(self.stdin.clone().map(HostValue::Bytes)),
                HostTraceFieldSensitivity::Secret,
            )
    }
}

impl HostTraceRequest for TcpConnectRequest {
    fn trace_payload(&self) -> HostTracePayload {
        let operation = match &self.operation {
            TcpConnectOperation::Connect { endpoint } => variant(
                "Connect",
                vec![
                    HostValue::String(endpoint.host.clone()),
                    HostValue::UInt(endpoint.port as u128),
                ],
            ),
        };
        HostTracePayload::new("tcp", "Net.tcp_connect").with_field(
            "operation",
            operation,
            HostTraceFieldSensitivity::Sensitive,
        )
    }
}

impl HostTraceRequest for StreamRequest {
    fn trace_payload(&self) -> HostTracePayload {
        let (action, operation) = match &self.operation {
            StreamOperation::Read {
                stream,
                max_bytes,
                timeout_ms,
            } => (
                "Stream.read",
                variant(
                    "Read",
                    vec![
                        byte_stream(stream),
                        HostValue::UInt(*max_bytes as u128),
                        option(timeout_ms.map(|value| HostValue::UInt(value as u128))),
                    ],
                ),
            ),
            StreamOperation::ReadUntilLimit {
                stream,
                limit_bytes,
                timeout_ms,
            } => (
                "Stream.read",
                variant(
                    "ReadUntilLimit",
                    vec![
                        byte_stream(stream),
                        HostValue::UInt(*limit_bytes as u128),
                        option(timeout_ms.map(|value| HostValue::UInt(value as u128))),
                    ],
                ),
            ),
            StreamOperation::WriteAll { stream, body } => (
                "Stream.write",
                variant(
                    "WriteAll",
                    vec![byte_stream(stream), HostValue::Bytes(body.clone())],
                ),
            ),
            StreamOperation::Flush { stream } => {
                ("Stream.flush", variant("Flush", vec![byte_stream(stream)]))
            }
            StreamOperation::Close { stream } => {
                ("Stream.close", variant("Close", vec![byte_stream(stream)]))
            }
        };
        HostTracePayload::new("stream", action).with_field(
            "operation",
            operation,
            HostTraceFieldSensitivity::Sensitive,
        )
    }
}

impl HostTraceRequest for TlsConnectRequest {
    fn trace_payload(&self) -> HostTracePayload {
        let operation = match &self.operation {
            TlsConnectOperation::Connect {
                stream,
                server_name,
            } => variant(
                "Connect",
                vec![tcp_stream(stream), HostValue::String(server_name.clone())],
            ),
        };
        HostTracePayload::new("tls", "Tls.handshake").with_field(
            "operation",
            operation,
            HostTraceFieldSensitivity::Sensitive,
        )
    }
}

impl HostTraceRequest for SecretRequest {
    fn trace_payload(&self) -> HostTracePayload {
        let (action, operation) = match &self.operation {
            SecretOperation::Read { key } => (
                "Secret.read",
                variant("Read", vec![HostValue::String(key.clone())]),
            ),
            SecretOperation::HmacSha256 { key, body } => (
                "Secret.use",
                variant(
                    "HmacSha256",
                    vec![
                        HostValue::String(key.id().to_owned()),
                        HostValue::Bytes(body.clone()),
                    ],
                ),
            ),
        };
        HostTracePayload::new("secret", action).with_field(
            "operation",
            operation,
            HostTraceFieldSensitivity::Secret,
        )
    }
}

impl HostTraceRequest for BrowserProtocolRequest {
    fn trace_payload(&self) -> HostTracePayload {
        let (action, operation) = match &self.operation {
            BrowserProtocolOperation::Attach { profile } => (
                "Browser.attach",
                variant("Attach", vec![HostValue::String(profile.clone())]),
            ),
            BrowserProtocolOperation::Create { profile } => (
                "Browser.create",
                variant("Create", vec![HostValue::String(profile.clone())]),
            ),
            BrowserProtocolOperation::Send { session, message } => (
                "Browser.send",
                variant(
                    "Send",
                    vec![
                        HostValue::String(session.clone()),
                        HostValue::Bytes(message.clone()),
                    ],
                ),
            ),
            BrowserProtocolOperation::Recv { session, max_bytes } => (
                "Browser.recv",
                variant(
                    "Recv",
                    vec![
                        HostValue::String(session.clone()),
                        HostValue::UInt(*max_bytes as u128),
                    ],
                ),
            ),
            BrowserProtocolOperation::Screenshot { session, max_bytes } => (
                "Browser.screenshot",
                variant(
                    "Screenshot",
                    vec![
                        HostValue::String(session.clone()),
                        HostValue::UInt(*max_bytes as u128),
                    ],
                ),
            ),
            BrowserProtocolOperation::Close { session } => (
                "Browser.close",
                variant("Close", vec![HostValue::String(session.clone())]),
            ),
        };
        HostTracePayload::new("browser", action).with_field(
            "operation",
            operation,
            HostTraceFieldSensitivity::Sensitive,
        )
    }
}

fn workspace_path(path: &WorkspacePathRef) -> HostValue {
    record([
        ("region", HostValue::String(path.region.as_str().to_owned())),
        (
            "relative",
            HostValue::String(path.relative.to_string_lossy().into_owned()),
        ),
    ])
}

fn byte_stream(stream: &ByteStreamRef) -> HostValue {
    stream_identity(stream.handle().identity_fingerprint(), stream.origin())
}

fn tcp_stream(stream: &TcpStreamRef) -> HostValue {
    stream_identity(stream.handle().identity_fingerprint(), stream.origin())
}

fn stream_identity(fingerprint: String, origin: &ByteStreamOrigin) -> HostValue {
    record([
        ("handle", HostValue::String(fingerprint)),
        ("origin", stream_origin(origin)),
    ])
}

fn stream_origin(origin: &ByteStreamOrigin) -> HostValue {
    match origin {
        ByteStreamOrigin::Tcp { host, port } => variant(
            "Tcp",
            vec![
                HostValue::String(host.clone()),
                HostValue::UInt(*port as u128),
            ],
        ),
        ByteStreamOrigin::Tls {
            host,
            port,
            server_name,
        } => variant(
            "Tls",
            vec![
                HostValue::String(host.clone()),
                HostValue::UInt(*port as u128),
                option(server_name.clone().map(HostValue::String)),
            ],
        ),
        ByteStreamOrigin::File { path } => variant("File", vec![HostValue::String(path.clone())]),
        ByteStreamOrigin::Browser { session } => {
            variant("Browser", vec![HostValue::String(session.clone())])
        }
        ByteStreamOrigin::Opaque => variant("Opaque", Vec::new()),
    }
}
