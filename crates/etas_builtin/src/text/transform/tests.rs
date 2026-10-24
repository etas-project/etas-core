use super::*;
use crate::{BuiltinTypeTag, call_pure_intrinsic};

#[test]
fn borrowed_transforms_match_unicode_empty_and_separator_kernel_semantics() {
    for (id, args, expected) in [
        (
            pure::TEXT_TRIM,
            vec!["\u{2003}中😀e\u{301}\n"],
            BuiltinValue::String("中😀e\u{301}".into()),
        ),
        (
            pure::TEXT_TRIM,
            vec!["unchanged"],
            BuiltinValue::String("unchanged".into()),
        ),
        (pure::TEXT_TRIM, vec![""], BuiltinValue::String("".into())),
        (
            pure::TEXT_LOWERCASE,
            vec!["İΣ"],
            BuiltinValue::String("i\u{307}ς".into()),
        ),
        (
            pure::TEXT_UPPERCASE,
            vec!["ß中"],
            BuiltinValue::String("SS中".into()),
        ),
        (
            pure::TEXT_LINES,
            vec!["a\r\nb\n\n"],
            strings(&["a", "b", ""]),
        ),
        (pure::TEXT_LINES, vec![""], strings(&[])),
        (
            pure::TEXT_SPLIT,
            vec!["中😀", ""],
            strings(&["", "中", "😀", ""]),
        ),
        (pure::TEXT_SPLIT, vec!["", ""], strings(&["", ""])),
        (pure::TEXT_SPLIT, vec!["aaa", "aa"], strings(&["", "a"])),
        (pure::TEXT_SPLIT, vec!["abc", "missing"], strings(&["abc"])),
    ] {
        let id = StdIntrinsicId(id);
        let transform = TextTransform::for_intrinsic(id).unwrap();
        assert_eq!(transform.evaluate(&args).unwrap().into_owned(), expected);
        let owned = args
            .iter()
            .map(|s| BuiltinValue::String((*s).into()))
            .collect::<Vec<_>>();
        assert_eq!(call_pure_intrinsic(id, &owned).unwrap(), expected);
        assert!(matches!(
            transform.evaluate(&[]),
            Err(BuiltinError::ArityMismatch { .. })
        ));
        for index in 0..owned.len() {
            let mut invalid = owned.clone();
            invalid[index] = BuiltinValue::Bool(true);
            assert_eq!(
                call_pure_intrinsic(id, &invalid),
                Err(BuiltinError::TypeMismatch {
                    expected: BuiltinTypeTag::String,
                    actual: BuiltinTypeTag::Bool,
                })
            );
        }
    }
}

fn strings(parts: &[&str]) -> BuiltinValue {
    BuiltinValue::Array(
        parts
            .iter()
            .map(|s| BuiltinValue::String((*s).into()))
            .collect(),
    )
}

#[test]
fn trim_and_parts_borrow_without_copying_text() {
    let text = "hello".repeat(4000);
    let TextOutput::String(Cow::Borrowed(result)) = TextTransform::Trim.evaluate(&[&text]).unwrap()
    else {
        panic!("borrowed trim")
    };
    assert_eq!(result.as_ptr(), text.as_ptr());
    let TextOutput::Array(mut parts) = TextTransform::Split
        .evaluate(&[&text, "not-found"])
        .unwrap()
    else {
        panic!("parts")
    };
    assert_eq!(parts.next().unwrap().as_ptr(), text.as_ptr());
    assert_eq!(parts.next(), None);
}
