module std.agent.message;

// Agent message support declaration.
public type AgentId = string;

// Agent message support declaration.
public type Message<T> = { body: T, content: T, id: std.agent.message.MessageId, from: std.option.Option<std.agent.message.Participant>, role: std.agent.message.Role, session: std.option.Option<std.agent.session.SessionId>, created_at: std.runtime.time.Time, provenance: std.option.Option<std.agent.message.Provenance> };

// Agent message support declaration.
public type MessageId = string;

// Agent message support declaration.
public type Participant = string;

// Agent message support declaration.
public type Provenance = { trace_id: Option<TraceId>, source: Option<string> };

// Agent message support declaration.
public enum Role {
    System,
    User,
    Assistant,
    Tool,
}

// Agent message support declaration.
public type TraceId = string;

impl Message<Source> {
    public flow cast<Target>(self) -> Option<Message<Target>> ![] {
        return match PayloadRepresentation.convert<Source, Target>(self.content) {
            None => None,
            Some(value) => Some(Message {
                body= value,
                content= value,
                id= self.id,
                from= self.from,
                role= self.role,
                session= self.session,
                created_at= self.created_at,
                provenance= self.provenance,
            }),
        };
    }
}

impl Message<T> {
    public flow new(content: T) -> Message<T> ![] {
        let context = MessageContext.capture();
        let message = Message {
            body= content,
            content= content,
            id= MessageId("msg-" + std.text.from_u64(context.sequence)),
            from= None,
            role= Role.User,
            session= context.session,
            created_at= context.created_at,
            provenance= Some(Provenance { trace_id= context.trace, source= Some("Message.new") }),
        };
        context.record(message);
        return message;
    }

    public flow with_session(message: Message<T>, config: std.agent.session.SessionConfig) -> Message<T> ![] {
        let session = perform std.effects.Session.resolve(config);
        var attached = message;
        attached.session = Some(session);
        // Append returns the backend-confirmed value, including deduplication.
        return perform std.effects.Session.append(attached);
    }
}
