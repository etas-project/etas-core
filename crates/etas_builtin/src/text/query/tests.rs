use super::*;
use crate::{BuiltinTypeTag, call_pure_intrinsic};

#[test]
fn borrowed_text_queries_keep_kernel_values_and_failures() {
    for (id, args, expected) in [
        (pure::TEXT_LEN, vec![""], BuiltinValue::Usize(0)),
        (
            pure::TEXT_LEN,
            vec!["中😀e\u{301}"],
            BuiltinValue::Usize(10),
        ),
        (
            pure::TEXT_CONTAINS,
            vec!["中😀e\u{301}", "😀"],
            BuiltinValue::Bool(true),
        ),
        (
            pure::TEXT_CONTAINS,
            vec!["abc", "z"],
            BuiltinValue::Bool(false),
        ),
        (
            pure::TEXT_STARTS_WITH,
            vec!["abc", ""],
            BuiltinValue::Bool(true),
        ),
        (
            pure::TEXT_STARTS_WITH,
            vec!["abc", "b"],
            BuiltinValue::Bool(false),
        ),
        (
            pure::TEXT_ENDS_WITH,
            vec!["a\n", "\n"],
            BuiltinValue::Bool(true),
        ),
        (
            pure::TEXT_ENDS_WITH,
            vec!["", "x"],
            BuiltinValue::Bool(false),
        ),
    ] {
        let id = StdIntrinsicId(id);
        let query = TextQuery::for_intrinsic(id).unwrap();
        assert_eq!(query.evaluate(&args).unwrap(), expected);
        let owned = args
            .iter()
            .map(|s| BuiltinValue::String((*s).into()))
            .collect::<Vec<_>>();
        assert_eq!(call_pure_intrinsic(id, &owned).unwrap(), expected);
        assert!(matches!(
            query.evaluate(&[]),
            Err(BuiltinError::ArityMismatch { .. })
        ));
        let mut invalid = owned;
        invalid[0] = BuiltinValue::Bool(false);
        assert_eq!(
            call_pure_intrinsic(id, &invalid),
            Err(BuiltinError::TypeMismatch {
                expected: BuiltinTypeTag::String,
                actual: BuiltinTypeTag::Bool
            })
        );
    }
    assert!(TextQuery::for_intrinsic(StdIntrinsicId(pure::HTTP_DECODE_RESPONSE)).is_none());
}
