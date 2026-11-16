module std.runtime.approval;

// Public declarations migrated from the legacy API inventory.
// Maintained as Etas source; native bindings retain existing execution during migration.

// Approval support type.
public type ApprovalDecision;

// Approval support type.
public type ApprovalRequest;

// Approval support type.
public type Risk;

// Request human approval through the future runtime boundary.
@std.binding("std.runtime.approval.approve")
public flow approve<T>(arg0: string, arg1: T, arg2: std.runtime.approval.Risk) -> bool ![Approval] {}
