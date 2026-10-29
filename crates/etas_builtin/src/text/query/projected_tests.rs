use super::*;

struct Segments(Vec<String>);

impl TextInput for Segments {
    type Chunks<'a> = std::iter::Map<std::slice::Iter<'a, String>, fn(&String) -> &str>;
    fn byte_len(&self) -> usize {
        self.0.iter().map(String::len).sum()
    }
    fn contiguous(&self) -> Option<&str> {
        None
    }
    fn chunks(&self) -> Self::Chunks<'_> {
        self.0.iter().map(String::as_str)
    }
}

fn splits(text: &str) -> impl Iterator<Item = Segments> + '_ {
    text.char_indices()
        .map(|(index, _)| index)
        .chain(std::iter::once(text.len()))
        .map(|index| {
            Segments(vec![
                String::new(),
                text[..index].into(),
                String::new(),
                text[index..].into(),
                String::new(),
            ])
        })
}

#[test]
fn projected_queries_match_flat_kernels_at_every_unicode_split() {
    let texts = [
        "",
        "a",
        "ababa",
        "abacabac",
        "中😀e\u{301}終",
        "\0é\n中",
        "😀😀中😀",
    ];
    for source in texts {
        for pattern in texts
            .into_iter()
            .chain(["ab", "aba", "bac", "😀", "e\u{301}", "\n"])
        {
            for source_parts in splits(source) {
                for pattern_parts in splits(pattern) {
                    for query in [
                        TextQuery::Contains,
                        TextQuery::StartsWith,
                        TextQuery::EndsWith,
                    ] {
                        let expected = query.evaluate(&[source, pattern]);
                        let actual = query.evaluate_projected(&[&source_parts, &pattern_parts]);
                        assert_eq!(actual, expected, "{query:?}: {source:?} / {pattern:?}");
                    }
                }
                assert_eq!(
                    TextQuery::Len.evaluate_projected(&[&source_parts]),
                    TextQuery::Len.evaluate(&[source])
                );
                for query in [
                    crate::collections::count::CountQuery::Len,
                    crate::collections::count::CountQuery::IsEmpty,
                ] {
                    assert_eq!(
                        query.evaluate_text_projected(&source_parts),
                        query.evaluate_text(source)
                    );
                }
            }
        }
    }
}

#[test]
fn projected_parser_preserves_standard_values_and_error_precedence() {
    let mut texts = vec![
        "".into(),
        "+".into(),
        "-".into(),
        "00".into(),
        "-0".into(),
        "00-1".into(),
        "00+1".into(),
        "中".into(),
        "٠".into(),
        " 1".into(),
        "1\n".into(),
        "2147483647".into(),
        "-2147483648".into(),
        "2147483648".into(),
        "-2147483649".into(),
        "2147483647x".into(),
        "-2147483648x".into(),
        "2147483648x".into(),
        "-2147483649x".into(),
        "999999999999x".into(),
        "1😀".into(),
        "-0002147483648".into(),
    ];
    for zeros in [1, 10, 1000, 4000] {
        for suffix in [
            "",
            "1",
            "2147483647",
            "2147483648",
            "2147483649x",
            "中",
            "-1",
        ] {
            for sign in ["", "+", "-"] {
                texts.push(format!("{sign}{}{suffix}", "0".repeat(zeros)));
            }
        }
    }
    for text in texts {
        let expected = TextQuery::ParseI32.evaluate(&[text.as_str()]);
        // Include each significant boundary, plus the middle of a long zero run.
        let points = text
            .char_indices()
            .map(|(index, _)| index)
            .filter(|index| *index < 16 || *index + 16 > text.len() || *index == text.len() / 2)
            .chain(std::iter::once(text.len()));
        for index in points {
            let parts = Segments(vec![
                text[..index].into(),
                String::new(),
                text[index..].into(),
            ]);
            assert_eq!(
                TextQuery::ParseI32.evaluate_projected(&[&parts]),
                expected,
                "{text:?} at {index}"
            );
        }
    }
}

#[test]
fn projected_queries_reject_wrong_arity_without_reading_inputs() {
    for query in [
        TextQuery::Len,
        TextQuery::Contains,
        TextQuery::StartsWith,
        TextQuery::EndsWith,
        TextQuery::ParseI32,
    ] {
        assert!(matches!(
            query.evaluate_projected::<Segments>(&[]),
            Err(BuiltinError::ArityMismatch { .. })
        ));
    }
}
