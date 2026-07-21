#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoweringHint {
    None,
    PureBuiltin,
    RuntimeCall,
    HostBoundary,
    ApprovalBoundary,
    ErrorRaise,
}
