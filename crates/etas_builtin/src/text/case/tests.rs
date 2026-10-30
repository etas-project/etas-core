use super::*;

struct Parts<'a>(&'a [&'a str]);

impl TextInput for Parts<'_> {
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

fn mapped(parts: &[&str], mapping: CaseMapping) -> String {
    match map_case(&Parts(parts), mapping).unwrap() {
        CaseOutput::Unchanged => parts.concat(),
        CaseOutput::Changed(output) => output,
    }
}

#[test]
fn split_case_mapping_preserves_all_unicode_scalar_expansions() {
    let samples = [
        "",
        "abc",
        "ABC",
        "中😀",
        "Straße İ ﬃ ǅ ΐ",
        "AΣ\u{301}A AΣ\u{200d}! ΣΣΣ AΣ\u{2b0}A",
    ];
    for source in samples {
        for split in source
            .char_indices()
            .map(|(index, _)| index)
            .chain([source.len()])
        {
            let parts = [&source[..split], "", &source[split..]];
            assert_eq!(
                mapped(&parts, CaseMapping::Lowercase),
                source.to_lowercase(),
                "split={split}: {source:?}"
            );
            assert_eq!(
                mapped(&parts, CaseMapping::Uppercase),
                source.to_uppercase(),
                "split={split}: {source:?}"
            );
        }
    }
}

#[test]
fn unchanged_case_results_can_reuse_the_callers_backing() {
    for (mapping, parts) in [
        (CaseMapping::Lowercase, ["abc中", "😀"]),
        (CaseMapping::Uppercase, ["ABC中", "😀"]),
        (CaseMapping::Lowercase, ["", ""]),
    ] {
        assert_eq!(
            map_case(&Parts(&parts), mapping).unwrap(),
            CaseOutput::Unchanged
        );
    }
}

#[test]
fn sigma_context_properties_match_the_rust_unicode_tables_for_every_scalar() {
    // The native flat path and the segmented path must agree even when a Rust
    // toolchain updates its Unicode tables. These contexts distinguish cased,
    // case-ignorable, and uncased boundaries, including overlapping properties.
    for raw in 0..=0x10ffff {
        let Some(ch) = char::from_u32(raw) else {
            continue;
        };
        let mut buffer = [0; 4];
        let scalar = ch.encode_utf8(&mut buffer);
        for (before, after) in [("AΣ", "A"), (".", "Σ")] {
            let parts = [before, scalar, after];
            let flat = parts.concat();
            assert_eq!(
                mapped(&parts, CaseMapping::Lowercase),
                flat.to_lowercase(),
                "Unicode context U+{raw:04X}, Rust {:?}, source {flat:?}",
                char::UNICODE_VERSION
            );
        }
    }
}
