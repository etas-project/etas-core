module std.agent.session;

// Checked session policy value.
public enum ContextPolicy {
    All,
    LastTurns(usize),
    SummaryPlusRecent(usize),
}

// Session support type.
public type Conversation = { session: std.option.Option<std.agent.session.SessionId>, messages: std.collections.Array<std.agent.message.Message<std.json.JsonValue>>, summary: std.option.Option<std.agent.session.SessionPublishedContext>, cursor: std.option.Option<string>, protocol_state: std.option.Option<std.agent.conversation.Protocol>, history_fence: std.option.Option<SessionHistoryFence> };

// Checked session policy value.
public enum RetentionPolicy {
    Days(usize),
    Forever,
}

// Session support type.
public type SessionConfig = { id: std.agent.session.SessionId, context: std.agent.session.ContextPolicy, retention: std.agent.session.RetentionPolicy };

// Bounded session history and backend-issued selection evidence.
public type SessionContextContent = { text: string, provenance: std.collections.Map<string, string> };

// Backend-confirmed context publication evidence.
public type SessionContextReceipt = { operation: std.memory.StorageOperationRef, session: string, generation: string, context_version: u64, durability: std.memory.StorageDurability };

// Confirmed context publication rejection.
public enum SessionContextRejection {
    Rejected(std.memory.StorageError),
    StaleHistory,
}

// Bounded session history and backend-issued selection evidence.
public type SessionCursor = { opaque: string };

// Bounded session history and backend-issued selection evidence.
public type SessionHistoryFence = { opaque: string };

// Bounded session history and backend-issued selection evidence.
public type SessionHistoryPage = { fence: std.agent.session.SessionHistoryFence, messages: std.collections.Array<std.agent.message.Message<std.json.JsonValue>>, published_context: std.option.Option<std.agent.session.SessionPublishedContext>, summary: std.option.Option<std.agent.session.SessionSummary>, cursor: std.option.Option<std.agent.session.SessionCursor> };

// Session support type.
public type SessionId = string;

// Bounded session history and backend-issued selection evidence.
public type SessionPublishedContext = { content: std.agent.session.SessionContextContent, fence: std.agent.session.SessionHistoryFence, version: u64 };

// Session support type.
public type SessionSummary = { text: string, message_count: usize };

// Read the current runtime session identifier.
@std.role("std.agent.session.current_session")
public flow current_session() -> std.agent.session.SessionId ![];

// Read one bounded page from an existing session using a stable backend cursor.
public flow history_page(config: SessionConfig, cursor: Option<SessionCursor>, limit: u32) -> SessionHistoryPage ![Error<std.memory.StorageError>] {
    return perform std.effects.Session.read(config, cursor, Some(limit));
}

impl std.agent.session.Conversation {
    public flow load(config: SessionConfig) -> Conversation ![Error<std.memory.StorageError>] {
        let session = perform std.effects.Session.resolve(config);
        var selected = config;
        selected.id = session;
        let page = perform std.effects.Session.read(selected, None, None);
        let cursor = match page.cursor {
            None => None,
            Some(value) => Some(value.opaque),
        };
        return Conversation {
            session= Some(session),
            messages= page.messages,
            summary= page.published_context,
            cursor= cursor,
            protocol_state= None,
            history_fence= Some(page.fence),
        };
    }
}

impl std.agent.session.SessionConfig {
    public flow continue_or_new<T ~ SessionKey>(key: T) -> SessionConfig ![] {
        return SessionConfig {
            id= SessionId("continue_or_new:" + key::SessionKey.stable_key()),
            context= ContextPolicy.All,
            retention= RetentionPolicy.Forever,
        };
    }
}
