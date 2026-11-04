use etas_builtin::text::case::{CaseMapping, CaseOutput, map_case};
use etas_builtin::text::unicode::{
    DATA_FINGERPRINT, NormalizationForm, UNICODE_VERSION, casefold, next_grapheme_boundary,
    normalize,
};
use std::collections::BTreeMap;

const NORMALIZATION_TESTS: &str = include_str!("unicode_data/NormalizationTest.txt");
const GRAPHEME_BREAK_TESTS: &str = include_str!("unicode_data/GraphemeBreakTest.txt");
const CASE_FOLDING: &str = include_str!("../src/text/unicode/data/source/CaseFolding.txt");
const UNICODE_DATA: &str = include_str!("../src/text/unicode/data/source/UnicodeData.txt");
const SPECIAL_CASING: &str = include_str!("../src/text/unicode/data/source/SpecialCasing.txt");

#[test]
fn generated_unicode_data_matches_the_standard_compatibility_descriptor() {
    let descriptor = etas_std::standard_registry().version().text.clone();
    assert_eq!(descriptor.unicode_version, UNICODE_VERSION);
    assert_eq!(
        descriptor.unicode_data_fingerprint,
        format!("{DATA_FINGERPRINT:016x}")
    );
}

fn scalars(text: &str) -> String {
    text.split_whitespace()
        .map(|scalar| {
            char::from_u32(u32::from_str_radix(scalar, 16).expect("Unicode scalar hex"))
                .expect("Unicode scalar value")
        })
        .collect()
}

#[test]
fn unicode_17_normalization_conformance() {
    for (line_number, line) in NORMALIZATION_TESTS.lines().enumerate() {
        let data = line.split('#').next().unwrap_or_default().trim();
        if data.is_empty() || data.starts_with('@') {
            continue;
        }
        let columns = data.split(';').take(5).map(scalars).collect::<Vec<_>>();
        assert_eq!(columns.len(), 5, "line {}", line_number + 1);
        let [source, nfc, nfd, nfkc, nfkd] = columns.as_slice() else {
            unreachable!()
        };

        for input in [source, nfc, nfd] {
            assert_eq!(
                normalize(input, NormalizationForm::Nfc).unwrap(),
                *nfc,
                "NFC line {}",
                line_number + 1
            );
            assert_eq!(
                normalize(input, NormalizationForm::Nfd).unwrap(),
                *nfd,
                "NFD line {}",
                line_number + 1
            );
        }
        for input in [source, nfc, nfd, nfkc, nfkd] {
            assert_eq!(
                normalize(input, NormalizationForm::Nfkc).unwrap(),
                *nfkc,
                "NFKC line {}",
                line_number + 1
            );
            assert_eq!(
                normalize(input, NormalizationForm::Nfkd).unwrap(),
                *nfkd,
                "NFKD line {}",
                line_number + 1
            );
        }
        assert_eq!(
            normalize(nfkc, NormalizationForm::Nfc).unwrap(),
            *nfkc,
            "NFC compatibility line {}",
            line_number + 1
        );
        assert_eq!(
            normalize(nfkd, NormalizationForm::Nfd).unwrap(),
            *nfkd,
            "NFD compatibility line {}",
            line_number + 1
        );
    }
}

#[test]
fn unicode_17_extended_grapheme_cluster_conformance() {
    for (line_number, line) in GRAPHEME_BREAK_TESTS.lines().enumerate() {
        let data = line.split('#').next().unwrap_or_default().trim();
        if data.is_empty() {
            continue;
        }

        let mut source = String::new();
        let mut expected = Vec::new();
        let mut marker = None;
        for token in data.split_whitespace() {
            match token {
                "÷" => marker = Some(true),
                "×" => marker = Some(false),
                scalar => {
                    if marker.take() == Some(true) {
                        expected.push(source.len());
                    }
                    source.push(
                        char::from_u32(
                            u32::from_str_radix(scalar, 16).expect("Unicode scalar hex"),
                        )
                        .expect("Unicode scalar value"),
                    );
                }
            }
        }
        if marker == Some(true) {
            expected.push(source.len());
        }

        let mut actual = vec![0];
        let mut offset = 0;
        while offset < source.len() {
            offset = next_grapheme_boundary(&source, offset);
            actual.push(offset);
        }
        assert_eq!(actual, expected, "grapheme line {}", line_number + 1);
    }
}

#[test]
fn unicode_17_default_full_casefold_conformance() {
    for (line_number, line) in CASE_FOLDING.lines().enumerate() {
        let data = line.split('#').next().unwrap_or_default().trim();
        if data.is_empty() {
            continue;
        }
        let columns = data.split(';').map(str::trim).collect::<Vec<_>>();
        if !matches!(columns.get(1), Some(&"C" | &"F")) {
            continue;
        }
        let input = scalars(columns[0]);
        let expected = scalars(columns[2]);
        assert_eq!(
            casefold(&input).unwrap(),
            expected,
            "casefold line {}",
            line_number + 1
        );
    }
}

fn generated_case_mappings() -> (BTreeMap<u32, String>, BTreeMap<u32, String>) {
    let mut lowercase = BTreeMap::new();
    let mut uppercase = BTreeMap::new();
    for line in UNICODE_DATA.lines().filter(|line| !line.is_empty()) {
        let fields = line.split(';').collect::<Vec<_>>();
        let scalar = u32::from_str_radix(fields[0], 16).unwrap();
        if !fields[13].is_empty() {
            lowercase.insert(scalar, scalars(fields[13]));
        }
        if !fields[12].is_empty() {
            uppercase.insert(scalar, scalars(fields[12]));
        }
    }
    for line in SPECIAL_CASING.lines() {
        let data = line.split('#').next().unwrap_or_default().trim();
        if data.is_empty() {
            continue;
        }
        let fields = data.split(';').map(str::trim).collect::<Vec<_>>();
        if !fields[4].is_empty() {
            continue;
        }
        let scalar = u32::from_str_radix(fields[0], 16).unwrap();
        lowercase.insert(scalar, scalars(fields[1]));
        uppercase.insert(scalar, scalars(fields[3]));
    }
    (lowercase, uppercase)
}

fn mapped_case(source: &str, mapping: CaseMapping) -> String {
    match map_case(source, mapping).unwrap() {
        CaseOutput::Unchanged => source.to_owned(),
        CaseOutput::Changed(output) => output,
    }
}

#[test]
fn unicode_17_default_case_mapping_conformance() {
    let (lowercase, uppercase) = generated_case_mappings();
    for (mapping, expected) in [
        (CaseMapping::Lowercase, lowercase),
        (CaseMapping::Uppercase, uppercase),
    ] {
        for (scalar, expected) in expected {
            let source = char::from_u32(scalar).unwrap().to_string();
            assert_eq!(
                mapped_case(&source, mapping),
                expected,
                "{mapping:?} U+{scalar:04X}"
            );
        }
    }
}
