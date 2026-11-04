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
        ("", "", ""),
        ("abc", "abc", "ABC"),
        ("ABC", "abc", "ABC"),
        ("中😀", "中😀", "中😀"),
        ("Straße İ ﬃ ǅ", "straße i\u{307} ﬃ ǆ", "STRASSE İ FFI Ǆ"),
        ("AΣA AΣ! ΣΣΣ", "aσa aς! σσς", "AΣA AΣ! ΣΣΣ"),
    ];
    for (source, lowercase, uppercase) in samples {
        for split in source
            .char_indices()
            .map(|(index, _)| index)
            .chain([source.len()])
        {
            let parts = [&source[..split], "", &source[split..]];
            assert_eq!(
                mapped(&parts, CaseMapping::Lowercase),
                lowercase,
                "split={split}: {source:?}"
            );
            assert_eq!(
                mapped(&parts, CaseMapping::Uppercase),
                uppercase,
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
fn final_sigma_uses_pinned_cased_and_case_ignorable_properties() {
    for (source, expected) in [
        ("AΣA", "aσa"),
        ("AΣ!", "aς!"),
        ("AΣ\u{301}A", "aσ\u{301}a"),
        ("AΣ\u{200d}!", "aς\u{200d}!"),
        ("ΣΣΣ", "σσς"),
    ] {
        assert_eq!(mapped(&[source], CaseMapping::Lowercase), expected);
    }
}
