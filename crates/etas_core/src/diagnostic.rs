use crate::{Span, TextRange};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub phase: DiagnosticPhase,
    pub severity: Severity,
    pub message: String,
    pub primary: DiagnosticSpan,
    pub labels: Vec<DiagnosticLabel>,
    pub notes: Vec<String>,
    pub help: Option<String>,
    pub suggestions: Vec<Suggestion>,
}

impl Diagnostic {
    pub fn syntax(code: SyntaxDiagnosticCode, span: Span, message: impl Into<String>) -> Self {
        let message = message.into();
        Self {
            code: DiagnosticCode::Syntax(code),
            phase: DiagnosticPhase::Parse,
            severity: Severity::Error,
            primary: DiagnosticSpan {
                span,
                label: Some(message.clone()),
            },
            labels: vec![DiagnosticLabel {
                span,
                style: LabelStyle::Primary,
                message: message.clone(),
            }],
            message,
            notes: Vec::new(),
            help: None,
            suggestions: Vec::new(),
        }
    }

    pub fn lex(code: SyntaxDiagnosticCode, span: Span, message: impl Into<String>) -> Self {
        let mut diagnostic = Self::syntax(code, span, message);
        diagnostic.phase = DiagnosticPhase::Lex;
        diagnostic
    }

    pub fn type_check(code: TypeDiagnosticCode, span: Span, message: impl Into<String>) -> Self {
        let message = message.into();
        Self {
            code: DiagnosticCode::Type(code),
            phase: DiagnosticPhase::TypeCheck,
            severity: Severity::Error,
            primary: DiagnosticSpan {
                span,
                label: Some(message.clone()),
            },
            labels: vec![DiagnosticLabel {
                span,
                style: LabelStyle::Primary,
                message: message.clone(),
            }],
            message,
            notes: Vec::new(),
            help: None,
            suggestions: Vec::new(),
        }
    }

    pub fn effect_check(
        code: EffectDiagnosticCode,
        span: Span,
        message: impl Into<String>,
    ) -> Self {
        let message = message.into();
        Self {
            code: DiagnosticCode::Effect(code),
            phase: DiagnosticPhase::EffectCheck,
            severity: Severity::Error,
            primary: DiagnosticSpan {
                span,
                label: Some(message.clone()),
            },
            labels: vec![DiagnosticLabel {
                span,
                style: LabelStyle::Primary,
                message: message.clone(),
            }],
            message,
            notes: Vec::new(),
            help: None,
            suggestions: Vec::new(),
        }
    }

    pub fn analysis(code: AnalysisDiagnosticCode, span: Span, message: impl Into<String>) -> Self {
        let message = message.into();
        Self {
            code: DiagnosticCode::Analysis(code),
            phase: DiagnosticPhase::Analysis,
            severity: Severity::Error,
            primary: DiagnosticSpan {
                span,
                label: Some(message.clone()),
            },
            labels: vec![DiagnosticLabel {
                span,
                style: LabelStyle::Primary,
                message: message.clone(),
            }],
            message,
            notes: Vec::new(),
            help: None,
            suggestions: Vec::new(),
        }
    }

    pub fn with_secondary(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(DiagnosticLabel {
            span,
            style: LabelStyle::Secondary,
            message: message.into(),
        });
        self
    }

    pub fn with_suggestion(mut self, suggestion: Suggestion) -> Self {
        self.suggestions.push(suggestion);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DiagnosticSpan {
    pub span: Span,
    pub label: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DiagnosticLabel {
    pub span: Span,
    pub style: LabelStyle,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LabelStyle {
    Primary,
    Secondary,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DiagnosticPhase {
    Lex,
    Parse,
    Lower,
    NameResolution,
    TypeCheck,
    EffectCheck,
    Analysis,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DiagnosticCode {
    Syntax(SyntaxDiagnosticCode),
    Name(NameDiagnosticCode),
    Type(TypeDiagnosticCode),
    Effect(EffectDiagnosticCode),
    Analysis(AnalysisDiagnosticCode),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SyntaxDiagnosticCode {
    UnexpectedToken,
    UnexpectedEof,
    MissingToken,
    UnclosedDelimiter,
    InvalidLiteral,
    UnterminatedString,
    UnterminatedBlockComment,
    InvalidItem,
    InvalidType,
    InvalidExpression,
    InvalidPattern,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NameDiagnosticCode {
    UnresolvedName,
    AmbiguousName,
    DuplicateSymbol,
    InvalidImportPath,
    UnsupportedQualifiedPath,
    ImportAliasConflict,
    InvalidImplTarget,
    UnresolvedEffectAction,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TypeDiagnosticCode {
    UnknownType,
    Mismatch,
    TypeMismatch,
    ArityMismatch,
    NonCallableCallee,
    WrongArgumentCount,
    MissingField,
    UnknownField,
    DuplicateField,
    BranchTypeMismatch,
    InvalidImplTargetKind,
    InvalidImplMemberKind,
    InvalidAnnotation,
    InvalidEffectArgument,
    EmptyHandler,
    HandlerArmArityMismatch,
    FinishOutsideHandler,
    FinishTypeMismatch,
    HandlerResultRequired,
    HandlerCompletionRequired,
    ReturnInsideHandlerArm,
    IncompleteTypeFacts,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EffectDiagnosticCode {
    UnknownEffectTag,
    UnresolvedPerformedAction,
    InvalidTryCapture,
    EscapedEffect,
    EffectOutsideDeclaredRow,
    HandlerProducedEffectOutsideDeclaredRow,
    MissingRequirement,
    MissingSandboxActionArgument,
    MissingOrInvalidLimit,
    MissingEffectLoopLimit,
    InvalidResume,
    ResumeOutsideHandler,
    ResumeUsedMoreThanOnce,
    CannotResumeNeverAction,
    ApprovalRequirementNotDominating,
    TraceSpecDenied,
    TraceSpecAllowViolation,
    TraceSpecAfterRequirementMissing,
    TraceSpecRequirementNotDominating,
    MissingHighImpactAcknowledgement,
    MissingExplicitEffectContract,
    MissingToolProviderBinding,
    RuntimeRequiredInPhase1,
    HandlerArmOutsideHandledRow,
    IncompleteEffectFacts,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AnalysisDiagnosticCode {
    MissingEntry,
    InvalidEntry,
    NonProgressingLoop,
    MissingCheckedFact,
    MissingHostHandler,
    UnsupportedPhase2RuntimeFeature,
    InvalidArguments,
    UnhandledRuntimeError,
    UnhandledEffectAction,
    ExecutionAborted,
    ExecutionNotImplemented,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Suggestion {
    pub title: String,
    pub edits: Vec<TextEdit>,
    pub applicability: Applicability,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TextEdit {
    pub range: TextRange,
    pub replacement: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Applicability {
    MachineApplicable,
    MaybeIncorrect,
    HasPlaceholders,
}
