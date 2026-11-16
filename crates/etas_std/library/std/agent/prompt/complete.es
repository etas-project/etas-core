module std.agent.prompt;

// Public declarations migrated from the legacy API inventory.
// Maintained as Etas source; native bindings retain existing execution during migration.

// Agent prompt support declaration.
@std.role("std.agent.prompt.Prompt")
public type Prompt;

// Agent prompt support declaration.
@std.role("std.agent.prompt.PromptEncode")
public spec PromptEncode;

// Agent prompt support declaration.
@std.role("std.agent.prompt.PromptPart")
public type PromptPart;

// Append assistant-channel content to a prompt value.
@std.binding("std.agent.prompt.assistant")
public flow assistant(arg0: std.agent.prompt.Prompt, arg1: string) -> std.agent.prompt.Prompt ![] {}

// Append data-channel content to a prompt value.
@std.binding("std.agent.prompt.data")
public flow data<T>(arg0: std.agent.prompt.Prompt, arg1: T) -> std.agent.prompt.Prompt ![] {}

// Construct an empty prompt value.
@std.binding("std.agent.prompt.new")
public flow new() -> std.agent.prompt.Prompt ![] {}

// Append trusted system-channel content to a prompt value.
@std.binding("std.agent.prompt.system")
public flow system(arg0: std.agent.prompt.Prompt, arg1: std.security.trust.Trusted<string>) -> std.agent.prompt.Prompt ![] {}

// Append user-channel content to a prompt value.
@std.binding("std.agent.prompt.user")
public flow user(arg0: std.agent.prompt.Prompt, arg1: std.security.trust.Public<string>) -> std.agent.prompt.Prompt ![] {}

impl std.agent.prompt.Prompt {
    public flow assistant(self, arg0: string) -> std.agent.prompt.Prompt ![] { return std.agent.prompt.assistant(self, arg0); }
    public flow data<T>(self, arg0: T) -> std.agent.prompt.Prompt ![] { return std.agent.prompt.data(self, arg0); }
    public flow new() -> std.agent.prompt.Prompt ![] { return std.agent.prompt.new(); }
    public flow system(self, arg0: std.security.trust.Trusted<string>) -> std.agent.prompt.Prompt ![] { return std.agent.prompt.system(self, arg0); }
    public flow user(self, arg0: std.security.trust.Public<string>) -> std.agent.prompt.Prompt ![] { return std.agent.prompt.user(self, arg0); }
}
