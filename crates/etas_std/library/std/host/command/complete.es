module std.host.command;

// Existential boxing retains the concrete WorkspacePath<R> and its checked R.
// This private marker is no ambient path, root handle, or authority grant.
spec CommandDirectory;
impl std.fs.WorkspacePath<R> ~ CommandDirectory {}

public type Command = {
    private program: string,
    private args: Array<string>,
    private environment: Map<string, string>,
    private cwd: Option<? ~ CommandDirectory>,
    private stdin: Option<bytes>,
};

public enum CommandEvent {
    Exited(i32, bytes, bytes),
    Output(bytes, bytes),
    Stopped(bytes, bytes),
}

// The host issues the process state. The reconnect identifier is data;
// reconnect checks the selected sandbox and current authority.
type CommandProcessState;
public type CommandHandle = {
    private reconnect_id: string,
    private process: CommandProcessState,
};

public type CommandResult = { exit_code: i32, stdout: bytes, stderr: bytes };

impl Command {
    public flow new(program: string, args: Array<string>) -> Command ![] {
        if program == "" { std.core.abort("command program must not be empty"); }
        return Command {
            program= program,
            args= args,
            environment= {},
            cwd= None,
            stdin= None,
        };
    }

    public flow with_cwd<R ~ std.fs.Region>(self, path: std.fs.WorkspacePath<R>) -> Command ![] {
        var result = self;
        let directory: ? ~ CommandDirectory = path;
        result.cwd = Some(directory);
        return result;
    }

    public flow with_env(self, environment: Map<string, string>) -> Command ![] {
        var result = self;
        result.environment = environment;
        return result;
    }

    public flow with_stdin(self, body: bytes) -> Command ![] {
        var result = self;
        result.stdin = Some(body);
        return result;
    }
}

impl CommandHandle {
    public flow id(self) -> string ![] {
        return self.reconnect_id;
    }

    public flow read(self, max_bytes: usize) -> CommandEvent ![] {
        return perform std.effects.Command.read(self, max_bytes);
    }

    public flow write(self, body: bytes) -> unit ![] {
        perform std.effects.Command.write(self, body);
    }

    public flow close_stdin(self) -> unit ![] {
        perform std.effects.Command.close_stdin(self);
    }

    public flow stop(self) -> CommandEvent ![] {
        return perform std.effects.Command.stop(self);
    }
}

public flow reconnect(id: string, sandbox: std.host.sandbox.SandboxProfile) -> CommandHandle ![] {
    return perform std.effects.Command.reconnect(id, sandbox);
}

public flow run(command: Command, sandbox: std.host.sandbox.SandboxProfile = std.host.sandbox.DefaultCommandSandbox) -> CommandResult ![] {
    return perform std.effects.Command.run(command, sandbox);
}

public flow start(command: Command, sandbox: std.host.sandbox.SandboxProfile) -> CommandHandle ![] {
    return perform std.effects.Command.start(command, sandbox);
}
