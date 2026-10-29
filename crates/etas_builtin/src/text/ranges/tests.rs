use super::*;

struct Text<'a>(Vec<&'a str>);
impl TextInput for Text<'_> {
    type Chunks<'a>
        = std::iter::Copied<std::slice::Iter<'a, &'a str>>
    where
        Self: 'a;
    fn byte_len(&self) -> usize {
        self.0.iter().map(|part| part.len()).sum()
    }
    fn contiguous(&self) -> Option<&str> {
        None
    }
    fn chunks(&self) -> Self::Chunks<'_> {
        self.0.iter().copied()
    }
}

fn boundaries(text: &str) -> impl Iterator<Item = usize> + '_ {
    text.char_indices()
        .map(|(index, _)| index)
        .chain(std::iter::once(text.len()))
}

#[test]
fn segmented_trim_and_lines_match_standard_text_at_every_scalar_boundary() {
    for text in [
        "",
        " ",
        "\u{2003}\t\r\n",
        "x",
        "  中😀  ",
        "\n",
        "\r",
        "a\r\n中\n\n😀\r",
        "\r\n\n",
        "a\r\r\n",
    ] {
        assert_eq!(&text[trim_range(text)], text.trim());
        assert_eq!(
            line_ranges(text)
                .map(|range| &text[range])
                .collect::<Vec<_>>(),
            text.lines().collect::<Vec<_>>()
        );
        for boundary in boundaries(text) {
            let input = Text(vec!["", &text[..boundary], "", &text[boundary..], ""]);
            assert_eq!(
                &text[trim_range(&input)],
                text.trim(),
                "{text:?} at {boundary}"
            );
            let actual: Vec<_> = line_ranges(&input).map(|range| &text[range]).collect();
            assert_eq!(
                actual,
                text.lines().collect::<Vec<_>>(),
                "{text:?} at {boundary}"
            );
        }
    }
}

#[test]
fn segmented_split_matches_empty_unicode_overlap_and_cross_chunk_separators() {
    for text in [
        "",
        "a",
        "aaa",
        "abababa",
        "aaabaaab",
        "中😀e\u{301}😀end",
        ":a::",
        "\r\n\n",
    ] {
        for separator in [
            "",
            "a",
            "aa",
            "aba",
            "😀",
            "e\u{301}😀",
            ":",
            "\r\n",
            "missing",
        ] {
            assert_eq!(
                split_ranges(text, separator)
                    .unwrap()
                    .map(|range| &text[range])
                    .collect::<Vec<_>>(),
                text.split(separator).collect::<Vec<_>>()
            );
            for source_boundary in boundaries(text) {
                for separator_boundary in boundaries(separator) {
                    let source = Text(vec![&text[..source_boundary], "", &text[source_boundary..]]);
                    let pattern = Text(vec![
                        &separator[..separator_boundary],
                        "",
                        &separator[separator_boundary..],
                    ]);
                    let actual: Vec<_> = split_ranges(&source, &pattern)
                        .unwrap()
                        .map(|range| &text[range])
                        .collect();
                    assert_eq!(
                        actual,
                        text.split(separator).collect::<Vec<_>>(),
                        "{text:?} / {separator:?} at {source_boundary}, {separator_boundary}"
                    );
                }
            }
        }
    }
}

#[test]
fn segmented_join_matches_flat_output_and_projection_errors() {
    use super::super::join::{JoinError, join_text_projected};
    for separator in ["", ":", "中😀"] {
        for boundary in boundaries(separator) {
            let separator = Text(vec![&separator[..boundary], &separator[boundary..]]);
            for parts in [vec![], vec![""], vec!["中", "😀", ""], vec!["a", "b", "c"]] {
                let inputs = parts
                    .iter()
                    .map(|part| Text(vec!["", part, ""]))
                    .collect::<Vec<_>>();
                let output =
                    join_text_projected(inputs.iter().map(Ok::<_, ()>), &separator).unwrap();
                assert_eq!(output, parts.join(&separator.0.concat()));
                assert_eq!(
                    join_text_projected([Ok(&separator), Err("bad")].into_iter(), &separator),
                    Err(JoinError::Projection("bad"))
                );
            }
        }
    }
}
