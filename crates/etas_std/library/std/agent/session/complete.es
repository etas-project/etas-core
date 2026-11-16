module std.agent.session;

// Public declarations migrated from the legacy API inventory.
// Maintained as Etas source; native bindings retain existing execution during migration.

// Checked session policy value.
@std.role("std.agent.session.ContextPolicy")
public enum ContextPolicy {
    All,
    LastTurns(usize),
    SummaryPlusRecent(usize),
}

// Session support type.
@std.role("std.agent.session.Conversation")
public type Conversation = { session: std.agent.session.SessionId, messages: std.collections.Array<std.agent.message.Message<std.json.JsonValue>>, summary: std.option.Option<std.agent.session.SessionPublishedContext>, cursor: std.option.Option<string> };

// Retain session history for a number of days.
@std.binding("std.agent.session.Days")
public flow Days(arg0: usize) -> std.agent.session.RetentionPolicy ![] {}

// Select the last N conversation turns as context.
@std.binding("std.agent.session.LastTurns")
public flow LastTurns(arg0: usize) -> std.agent.session.ContextPolicy ![] {}

// Checked session policy value.
@std.role("std.agent.session.RetentionPolicy")
public enum RetentionPolicy {
    Days(usize),
    Forever,
}

// Session support type.
@std.role("std.agent.session.SessionConfig")
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
@std.role("std.agent.session.SessionId")
public type SessionId = string;

// Bounded session history and backend-issued selection evidence.
public type SessionPublishedContext = { content: std.agent.session.SessionContextContent, fence: std.agent.session.SessionHistoryFence, version: u64 };

// Session support type.
public type SessionSummary = { text: string, message_count: usize };

// Select summarized history plus recent turns as context.
@std.binding("std.agent.session.SummaryPlusRecent")
public flow SummaryPlusRecent(arg0: usize) -> std.agent.session.ContextPolicy ![] {}

// Continue an existing session identified by a stable key, or create one.
@std.binding("std.agent.session.continue_or_new")
public flow continue_or_new<T>(arg0: T) -> std.agent.session.SessionConfig ![] {}

// Read the current runtime session identifier.
@std.binding("std.agent.session.current_session")
public flow current_session() -> std.agent.session.SessionId ![] {}

// Read one bounded page from an existing session using a stable backend cursor.
@std.binding("std.agent.session.history_page")
public flow history_page(arg0: std.agent.session.SessionConfig, arg1: std.option.Option<std.agent.session.SessionCursor>, arg2: u32) -> std.agent.session.SessionHistoryPage ![Error<std.memory.StorageError>] {}

// Load selected conversation history for a session.
@std.binding("std.agent.session.load")
public flow load(arg0: std.agent.session.SessionConfig) -> std.agent.session.Conversation ![] {}

// Prepare, conditionally publish, or reconcile caller-produced session context.
@std.binding("std.agent.session.prepare_context")
public flow prepare_context(arg0: std.agent.session.SessionConfig, arg1: std.agent.session.SessionHistoryFence, arg2: std.agent.session.SessionContextContent) -> std.memory.StorageOperationRef ![Error<std.memory.StorageError>] {}

// Prepare, conditionally publish, or reconcile caller-produced session context.
@std.binding("std.agent.session.publish_context")
public flow publish_context(arg0: std.agent.session.SessionConfig, arg1: std.agent.session.SessionHistoryFence, arg2: std.agent.session.SessionContextContent, arg3: std.memory.StorageOperationRef) -> std.memory.WriteOutcome<std.agent.session.SessionContextReceipt, std.agent.session.SessionContextRejection> ![Error<std.memory.StorageError>] {}

// Prepare, conditionally publish, or reconcile caller-produced session context.
@std.binding("std.agent.session.reconcile_context")
public flow reconcile_context(arg0: std.agent.session.SessionConfig, arg1: std.memory.StorageOperationRef) -> std.memory.ReconcileResult<std.agent.session.SessionContextReceipt, std.agent.session.SessionContextRejection> ![Error<std.memory.StorageError>] {}

impl std.agent.session.Conversation {
    public flow load(arg0: std.agent.session.SessionConfig) -> std.agent.session.Conversation ![] { return std.agent.session.load(arg0); }
}

impl std.agent.session.SessionConfig {
    public flow continue_or_new<T>(arg0: T) -> std.agent.session.SessionConfig ![] { return std.agent.session.continue_or_new(arg0); }
}
