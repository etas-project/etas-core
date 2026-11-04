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
fn projected_queries_reject_wrong_arity_without_reading_inputs() {
    for query in [
        TextQuery::Len,
        TextQuery::Contains,
        TextQuery::StartsWith,
        TextQuery::EndsWith,
    ] {
        assert!(matches!(
            query.evaluate_projected::<Segments>(&[]),
            Err(BuiltinError::ArityMismatch { .. })
        ));
    }
}
