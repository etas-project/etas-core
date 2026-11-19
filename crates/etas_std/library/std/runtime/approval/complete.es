module std.runtime.approval;

public enum Risk {
    Low,
    Medium,
    High,
}

public type ApprovalRequest<T> = { title: string, content: T, risk: Risk };

public enum ApprovalDecision {
    Accepted,
    Rejected,
}

public flow approve<T>(title: string, content: T, risk: Risk) -> bool ![std.effects.Approval] {
    let decision = perform std.effects.Approval.request(ApprovalRequest {
        title= title,
        content= content,
        risk= risk,
    });
    return match decision {
        ApprovalDecision.Accepted => true,
        ApprovalDecision.Rejected => false,
    };
}
