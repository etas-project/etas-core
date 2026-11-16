module std.agent.message;

// Public declarations migrated from the legacy API inventory.
// Maintained as Etas source; native bindings retain existing execution during migration.

// Agent message support declaration.
public type AgentId;

// Agent message support declaration.
@std.role("std.agent.message.Message")
public type Message<T> = { body: T, content: T, id: std.agent.message.MessageId, from: std.option.Option<std.agent.message.Participant>, role: std.agent.message.Role, session: std.option.Option<std.agent.session.SessionId>, created_at: std.runtime.time.Time, provenance: std.option.Option<std.agent.message.Provenance> };

// Agent message support declaration.
public type MessageId;

// Agent message support declaration.
public type Participant;

// Agent message support declaration.
@std.role("std.agent.message.Provenance")
public type Provenance;

// Agent message support declaration.
public type Role;

// Agent message support declaration.
public type TraceId;

// Attempt a checked identity typed message cast.
@std.binding("std.agent.message.cast")
public flow cast<Source, Target>(arg0: std.agent.message.Message<Source>) -> std.option.Option<std.agent.message.Message<Target>> ![] {}

// Construct a typed message from a payload.
@std.binding("std.agent.message.new")
public flow new<T>(arg0: T) -> std.agent.message.Message<T> ![] {}

// Attach a runtime session configuration to a typed message.
@std.binding("std.agent.message.with_session")
public flow with_session<T>(arg0: std.agent.message.Message<T>, arg1: std.agent.session.SessionConfig) -> std.agent.message.Message<T> ![] {}

impl std.agent.message.Message<Source> {
    public flow cast<Target>(self) -> std.option.Option<std.agent.message.Message<Target>> ![] { return std.agent.message.cast(self); }
}

impl std.agent.message.Message<T> {
    public flow new(arg0: T) -> std.agent.message.Message<T> ![] { return std.agent.message.new(arg0); }
    public flow with_session(arg0: std.agent.message.Message<T>, arg1: std.agent.session.SessionConfig) -> std.agent.message.Message<T> ![] { return std.agent.message.with_session(arg0, arg1); }
}
