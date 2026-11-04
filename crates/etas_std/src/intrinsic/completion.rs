use super::{StdIntrinsicId, pure, runtime};

#[derive(Clone, Copy, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub enum IntrinsicCompletionSemantics {
    /// The operation returns a checked value, including Result-based failures.
    /// Resource exhaustion remains an invocation failure, not a language result.
    Returns,
    /// After arguments complete, the adapter returns or raises a checked
    /// declared error. Operational stop/fault remains an invocation failure.
    ReturnsOrRaisesDeclaredErrors,
}

pub fn intrinsic_completion_semantics(id: StdIntrinsicId) -> Option<IntrinsicCompletionSemantics> {
    if matches!(
        id.0,
        pure::TEXT_FIND_FROM
            | pure::TEXT_BYTE_SLICE
            | pure::TEXT_CHAR_SLICE
            | pure::TEXT_REPLACE_RANGE
            | pure::TEXT_REPEAT
            | pure::TEXT_GRAPHEME_SLICE
    ) {
        return Some(IntrinsicCompletionSemantics::ReturnsOrRaisesDeclaredErrors);
    }
    if matches!(
        id.0,
        pure::TEXT_TRIM..=pure::TEXT_ENDS_WITH
            | pure::TEXT_LEN
            | pure::TEXT_SPLIT..=pure::TEXT_JOIN
            | pure::TEXT_CHAR_COUNT..=pure::TEXT_REVERSE_CHARS
            | pure::TEXT_PARSE_I8..=pure::TEXT_FORMAT
            | pure::TEXT_GRAPHEMES..=pure::TEXT_CASEFOLD
            | pure::CHAR_IS_WHITESPACE..=pure::CHAR_FROM_U32
    ) {
        return Some(IntrinsicCompletionSemantics::Returns);
    }
    match id.0 {
        runtime::IO_READ_ALL
        | runtime::IO_READ_LINE
        | runtime::IO_PRINT
        | runtime::IO_PRINTLN
        | runtime::IO_EPRINTLN
        | runtime::SECRET_READ
        | runtime::BROWSER_ATTACH
        | runtime::BROWSER_CREATE
        | runtime::BROWSER_SEND
        | runtime::BROWSER_RECV
        | runtime::BROWSER_CLOSE
        | runtime::BROWSER_SCREENSHOT
        | runtime::CODE_CHECK
        | runtime::CODE_COMPILE => {
            Some(IntrinsicCompletionSemantics::ReturnsOrRaisesDeclaredErrors)
        }
        runtime::CODE_PARSE_BODY
        | runtime::CODE_FORMAT
        | runtime::CODE_WITH_BODY
        | runtime::CODE_DIFF
        | runtime::CHECKPOINT
        | pure::LIST_LEN
        | pure::LIST_IS_EMPTY
        | pure::LIMIT_ITERATIONS
        | pure::LIMIT_TOKENS
        | pure::LIMIT_CONTEXT_TOKENS
        | pure::LIMIT_COST
        | pure::LIMIT_WALL_TIME
        | pure::LIMIT_ATTEMPTS => Some(IntrinsicCompletionSemantics::Returns),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn io_protocol_completion_does_not_mark_partial_kernels_total() {
        assert_eq!(
            intrinsic_completion_semantics(StdIntrinsicId(runtime::IO_PRINTLN)),
            Some(IntrinsicCompletionSemantics::ReturnsOrRaisesDeclaredErrors)
        );
        assert_eq!(
            intrinsic_completion_semantics(StdIntrinsicId(runtime::CODE_WITH_BODY)),
            Some(IntrinsicCompletionSemantics::Returns)
        );
        assert_eq!(
            intrinsic_completion_semantics(StdIntrinsicId(runtime::CODE_DIFF)),
            Some(IntrinsicCompletionSemantics::Returns)
        );
        assert_eq!(
            intrinsic_completion_semantics(StdIntrinsicId(runtime::CODE_COMPILE)),
            Some(IntrinsicCompletionSemantics::ReturnsOrRaisesDeclaredErrors)
        );
        assert_eq!(
            intrinsic_completion_semantics(StdIntrinsicId(crate::intrinsic::pure::OPTION_UNWRAP)),
            None
        );
        for id in [
            pure::LIST_LEN,
            pure::LIST_IS_EMPTY,
            pure::TEXT_TRIM,
            pure::TEXT_LOWERCASE,
            pure::TEXT_UPPERCASE,
            pure::LIMIT_ITERATIONS,
            pure::LIMIT_TOKENS,
            pure::LIMIT_CONTEXT_TOKENS,
            pure::LIMIT_COST,
            pure::LIMIT_WALL_TIME,
            pure::LIMIT_ATTEMPTS,
        ] {
            assert_eq!(
                intrinsic_completion_semantics(StdIntrinsicId(id)),
                Some(IntrinsicCompletionSemantics::Returns)
            );
        }
        assert_eq!(
            intrinsic_completion_semantics(StdIntrinsicId(runtime::CHECKPOINT)),
            Some(IntrinsicCompletionSemantics::Returns)
        );
    }

    #[test]
    fn mediated_secret_and_browser_calls_preserve_declared_error_completion() {
        for id in [
            runtime::SECRET_READ,
            runtime::BROWSER_ATTACH,
            runtime::BROWSER_CREATE,
            runtime::BROWSER_SEND,
            runtime::BROWSER_RECV,
            runtime::BROWSER_CLOSE,
            runtime::BROWSER_SCREENSHOT,
        ] {
            assert_eq!(
                intrinsic_completion_semantics(StdIntrinsicId(id)),
                Some(IntrinsicCompletionSemantics::ReturnsOrRaisesDeclaredErrors)
            );
        }
    }
}
