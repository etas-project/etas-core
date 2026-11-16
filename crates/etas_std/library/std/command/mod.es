module std.command;

public import std.host.command.{Command, CommandResult};
public import std.host.sandbox.{SandboxProfile, DefaultCommandSandbox};


public flow run(command: Command, sandbox: SandboxProfile = DefaultCommandSandbox) -> CommandResult ![] {
    return std.host.command.run(command, sandbox);
}
