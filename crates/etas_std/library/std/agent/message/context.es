module std.agent.message;

// Execution context capture supplies only the fresh ordinal, clock, current
// session and trace identity. Message assembly and payload handling are source.
type MessageContext = {
    private sequence: u64,
    private session: Option<std.agent.session.SessionId>,
    private created_at: std.runtime.time.Time,
    private trace: Option<TraceId>,
};

impl MessageContext {
    @std.role("std.agent.message.MessageContext.capture")
    flow capture() -> MessageContext ![];

    // Record the source-assembled message in the runtime workflow event log.
    @std.role("std.agent.message.MessageContext.record")
    flow record<T>(self, message: Message<T>) -> unit ![];
}

// The VM checks an ordinary payload against a retained target type using the
// same neutral host-value representation as ingress. Execution identities,
// capabilities and protected values cannot be manufactured by conversion.
type PayloadRepresentation = {};

impl PayloadRepresentation {
    @std.role("std.agent.message.PayloadRepresentation.convert")
    flow convert<S, T>(value: S) -> Option<T> ![];
}
