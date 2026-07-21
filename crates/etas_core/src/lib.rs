pub mod arena;
pub mod diagnostic;
pub mod id;
pub mod interner;
pub mod line_index;
pub mod result;
pub mod source;
pub mod span;

#[doc(hidden)]
pub use serde;

pub use arena::Arena;
pub use diagnostic::{
    AnalysisDiagnosticCode, Applicability, Diagnostic, DiagnosticCode, DiagnosticLabel,
    DiagnosticPhase, DiagnosticSpan, EffectDiagnosticCode, LabelStyle, NameDiagnosticCode,
    Severity, Suggestion, SyntaxDiagnosticCode, TextEdit, TypeDiagnosticCode,
};
pub use id::Idx;
pub use interner::{InternedString, StringInterner};
pub use line_index::{LineCol, LineIndex};
pub use result::{DiagnosticSink, EtasError, EtasResult};
pub use source::{SourceFile, SourceId};
pub use span::{Span, TextRange, TextSize};
