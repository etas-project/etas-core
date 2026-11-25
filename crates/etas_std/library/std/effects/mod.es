module std.effects;

import std.fs.{Region, WorkspacePath, FsStat};


// Rust binds execution tags and host capabilities; source owns declarations.

@std.role("std.effects.Agentic")
public effect Agentic {
    @default_handler
    action infer<C, O>(arg0: std.agent.prompt.Prompt, arg1: std.agent.schema.Schema<O>) -> O ![];
    @default_handler
    action step<C, O>(conversation: std.agent.session.Conversation, schema: std.agent.schema.Schema<O>) -> std.agent.StepResult<C, O> ![];
}

@std.role("std.effects.Network")
public effect Network;

@std.role("std.effects.FileIO")
public effect FileIO;

@std.role("std.effects.Console")
public effect Console extends FileIO {
    @default_handler
    action stdin_read_line() -> string ![Error<std.io.IOError>];
    @default_handler
    action stdin_read_all() -> string ![Error<std.io.IOError>];
    @default_handler
    action stdout_write(text: string) -> unit ![Error<std.io.IOError>];
    @default_handler
    action stderr_write(text: string) -> unit ![Error<std.io.IOError>];
}

@std.role("std.effects.Command")
public effect Command {
    @default_handler
    action read<Capability0: std.host.command.CommandHandle>(handle: std.host.command.CommandHandle, max_bytes: usize) -> std.host.command.CommandEvent ![];
    @default_handler
    action reconnect<Capability0: std.host.sandbox.SandboxProfile>(id: string, sandbox: std.host.sandbox.SandboxProfile) -> std.host.command.CommandHandle ![];
    @default_handler
    action run<Capability0: std.host.sandbox.SandboxProfile>(command: std.host.command.Command, sandbox: std.host.sandbox.SandboxProfile) -> std.host.command.CommandResult ![];
    @default_handler
    action start<Capability0: std.host.sandbox.SandboxProfile>(command: std.host.command.Command, sandbox: std.host.sandbox.SandboxProfile) -> std.host.command.CommandHandle ![];
    @default_handler
    action stop<Capability0: std.host.command.CommandHandle>(arg0: std.host.command.CommandHandle) -> std.host.command.CommandEvent ![];
    @default_handler
    action write<Capability0: std.host.command.CommandHandle>(handle: std.host.command.CommandHandle, body: bytes) -> unit ![];
    @default_handler
    action close_stdin<Capability0: std.host.command.CommandHandle>(handle: std.host.command.CommandHandle) -> unit ![];
}

@std.role("std.effects.Memory")
public effect Memory {
    @default_handler
    action<K, V> read<Place: std.memory.MemoryPlace>(store: std.memory.Store<K, V>, request: std.memory.ReadRequest<bytes>) -> std.memory.ReadResult<bytes, bytes> ![Error<std.memory.StorageError>];
    @default_handler
    action<K, V> write<Place: std.memory.MemoryPlace>(store: std.memory.Store<K, V>, operation: std.memory.StorageOperationRef, request: std.memory.WriteRequest<bytes, bytes>, wire: string) -> std.memory.WriteOutcome<std.memory.MemoryWriteReceipt<bytes>, std.memory.MemoryWriteRejection> ![Error<std.memory.StorageError>];
    @default_handler
    action<K, V> reconcile<Place: std.memory.MemoryPlace>(store: std.memory.Store<K, V>, operation: std.memory.StorageOperationRef) -> std.memory.ReconcileResult<std.memory.MemoryWriteReceipt<bytes>, std.memory.MemoryWriteRejection> ![Error<std.memory.StorageError>];
}

@std.role("std.effects.Secret")
public effect Secret {
    action<T, U> declassify(value: std.security.trust.Secret<T>, strategy: std.security.declassify.DeclassificationStrategy<T, U>, reason: string) -> std.security.trust.Sanitized<U>;
    @default_handler
    action read<K>(arg0: std.secret.SecretKey<K>) -> std.secret.SecretValue<K> ![Error<std.secret.SecretError>];
    @default_handler
    action use<K>(key: std.secret.SecretValue<K>, body: bytes) -> std.crypto.Digest ![Error<std.crypto.CryptoError>];
}

@std.role("std.effects.Time")
public effect Time;

@std.role("std.effects.Human")
public effect Human;

@std.role("std.effects.Error")
public effect Error<Failure> {
    action raise<E>(error: E) -> never;
}

/// Requests a value of the given type from the active Handler.
public effect Inject<T> {
    action get() -> T;
}

@std.role("std.effects.Fs")
public effect Fs extends FileIO {
    @default_handler
    action open<R ~ Region>(path: WorkspacePath<R>, mode: std.fs.FileMode) -> std.fs.FileStream<R> ![Error<std.fs.IOError>];
    @default_handler
    action sync<R ~ Region>(file: std.fs.FileStream<R>) -> unit ![Error<std.fs.IOError>];
    @default_handler
    action canonicalize<R ~ Region>(path: WorkspacePath<R>) -> WorkspacePath<R> ![Error<std.fs.IOError>];
    @default_handler
    action read<R ~ Region>(path: WorkspacePath<R>, max_bytes: Option<usize>) -> Result<bytes, std.fs.ReadLimitError> ![Error<std.fs.IOError>];
    @default_handler
    action write<R ~ Region>(path: WorkspacePath<R>, contents: bytes) -> unit ![Error<std.fs.IOError>];
    @default_handler
    action list<R ~ Region>(path: WorkspacePath<R>, limits: Option<(usize, usize)>) -> Result<List<WorkspacePath<R>>, std.fs.ListLimitError> ![Error<std.fs.IOError>];
    @default_handler
    action stat<R ~ Region>(path: WorkspacePath<R>) -> FsStat ![Error<std.fs.IOError>];
    @default_handler
    action atomic_replace<R ~ Region>(path: WorkspacePath<R>, contents: bytes) -> unit ![Error<std.fs.IOError>];
    @default_handler
    action create_dir<R ~ Region>(path: WorkspacePath<R>, recursive: bool) -> unit ![Error<std.fs.IOError>];
    @default_handler
    action rename<R ~ Region>(source: WorkspacePath<R>, destination: WorkspacePath<R>) -> unit ![Error<std.fs.IOError>];
    @default_handler
    action remove_file<R ~ Region>(path: WorkspacePath<R>) -> unit ![Error<std.fs.IOError>];
    @default_handler
    action remove_dir<R ~ Region>(path: WorkspacePath<R>) -> unit ![Error<std.fs.IOError>];
}
