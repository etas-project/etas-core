use icu_properties::{
    CodePointSetData,
    props::{CaseIgnorable, Cased},
};

use super::input::TextInput;
use crate::BuiltinError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaseMapping {
    Lowercase,
    Uppercase,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CaseOutput {
    Unchanged,
    Changed(String),
}

#[derive(Clone)]
enum MappedChars {
    Lower(std::char::ToLowercase),
    Upper(std::char::ToUppercase),
}

impl Iterator for MappedChars {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        match self {
            Self::Lower(chars) => chars.next(),
            Self::Upper(chars) => chars.next(),
        }
    }
}

impl CaseMapping {
    fn scalar(self, ch: char) -> MappedChars {
        match self {
            Self::Lowercase => MappedChars::Lower(ch.to_lowercase()),
            Self::Uppercase => MappedChars::Upper(ch.to_uppercase()),
        }
    }
}

/// Sizes the complete output before writing. Unchanged results let the caller
/// share its original immutable backing; changed results own only output bytes.
pub fn map_case(
    input: &(impl TextInput + ?Sized),
    mapping: CaseMapping,
) -> Result<CaseOutput, BuiltinError> {
    let mut bytes = 0usize;
    let mut changed = false;
    let mut has_sigma = false;
    for ch in input.chunks().flat_map(str::chars) {
        has_sigma |= ch == 'Σ';
        let mapped = mapping.scalar(ch);
        changed |= !mapped.clone().eq(std::iter::once(ch));
        for ch in mapped {
            bytes = bytes
                .checked_add(ch.len_utf8())
                .filter(|bytes| *bytes <= isize::MAX as usize)
                .ok_or(BuiltinError::NumericOverflow)?;
        }
    }
    if !changed {
        return Ok(CaseOutput::Unchanged);
    }
    if let Some(flat) = input.contiguous() {
        return Ok(CaseOutput::Changed(match mapping {
            CaseMapping::Lowercase => flat.to_lowercase(),
            CaseMapping::Uppercase => flat.to_uppercase(),
        }));
    }
    let mut output = String::new();
    output
        .try_reserve_exact(bytes)
        .map_err(|_| BuiltinError::NumericOverflow)?;
    let chars = input.chunks().flat_map(str::chars);
    match mapping {
        CaseMapping::Lowercase if has_sigma => lowercase(chars, &mut output),
        CaseMapping::Lowercase => output.extend(chars.flat_map(char::to_lowercase)),
        CaseMapping::Uppercase => output.extend(chars.flat_map(char::to_uppercase)),
    }
    Ok(CaseOutput::Changed(output))
}

fn lowercase(chars: impl Iterator<Item = char>, output: &mut String) {
    let cased = CodePointSetData::new::<Cased>();
    let ignorable = CodePointSetData::new::<CaseIgnorable>();
    let mut preceded_by_cased = false;
    let mut pending_sigma = None;
    for ch in chars {
        if !ignorable.contains(ch) {
            let following_is_cased = cased.contains(ch);
            resolve_sigma(output, &mut pending_sigma, following_is_cased);
            if ch == 'Σ' && preceded_by_cased {
                pending_sigma = Some(output.len());
            }
            preceded_by_cased = following_is_cased;
        }
        output.extend(ch.to_lowercase());
    }
    resolve_sigma(output, &mut pending_sigma, false);
}

fn resolve_sigma(output: &mut String, pending: &mut Option<usize>, followed_by_cased: bool) {
    if let Some(offset) = pending.take()
        && !followed_by_cased
    {
        // Both forms have the same UTF-8 width. Each pending interval is
        // disjoint, so resolving a long ignorable suffix never restarts a scan.
        output.replace_range(offset..offset + 'σ'.len_utf8(), "ς");
    }
}

#[cfg(test)]
#[path = "case/tests.rs"]
mod tests;
