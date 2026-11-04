use super::input::TextInput;
use super::output::TextOutputWork;
use super::unicode;
use crate::{
    BuiltinError,
    work::{WorkControl, WorkPoll},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaseMapping {
    Lowercase,
    Uppercase,
    AsciiLowercase,
    AsciiUppercase,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CaseOutput {
    Unchanged,
    Changed(String),
}

#[derive(Clone)]
enum MappedChars {
    Mapped(std::slice::Iter<'static, u32>),
    One(std::option::IntoIter<char>),
}

impl Iterator for MappedChars {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        match self {
            Self::Mapped(chars) => chars
                .next()
                .copied()
                .map(|scalar| char::from_u32(scalar).expect("generated Unicode scalar")),
            Self::One(chars) => chars.next(),
        }
    }
}

impl CaseMapping {
    fn scalar(self, ch: char) -> MappedChars {
        match self {
            Self::Lowercase => unicode::lowercase_mapping(ch)
                .map_or_else(|| MappedChars::One(Some(ch).into_iter()), mapped),
            Self::Uppercase => unicode::uppercase_mapping(ch)
                .map_or_else(|| MappedChars::One(Some(ch).into_iter()), mapped),
            Self::AsciiLowercase => MappedChars::One(Some(ch.to_ascii_lowercase()).into_iter()),
            Self::AsciiUppercase => MappedChars::One(Some(ch.to_ascii_uppercase()).into_iter()),
        }
    }
}

fn mapped(values: &'static [u32]) -> MappedChars {
    MappedChars::Mapped(values.iter())
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
    let mut output = String::new();
    output
        .try_reserve_exact(bytes)
        .map_err(|_| BuiltinError::NumericOverflow)?;
    let chars = input.chunks().flat_map(str::chars);
    match mapping {
        CaseMapping::Lowercase if has_sigma => lowercase(chars, &mut output),
        CaseMapping::Lowercase | CaseMapping::Uppercase => {
            output.extend(chars.flat_map(|ch| mapping.scalar(ch)))
        }
        CaseMapping::AsciiLowercase | CaseMapping::AsciiUppercase => {
            output.extend(chars.flat_map(|ch| mapping.scalar(ch)))
        }
    }
    Ok(CaseOutput::Changed(output))
}

#[derive(Clone, Debug)]
pub struct TextCaseWork {
    mapping: CaseMapping,
    output: TextOutputWork,
    pending: Vec<char>,
    pending_index: usize,
    preceded_by_cased: bool,
    pending_sigma: Option<usize>,
}

impl TextCaseWork {
    pub fn new(mapping: CaseMapping) -> Self {
        Self {
            mapping,
            output: TextOutputWork::default(),
            pending: Vec::new(),
            pending_index: 0,
            preceded_by_cased: false,
            pending_sigma: None,
        }
    }

    pub fn reserved_bytes(&self) -> usize {
        self.output.reserved_bytes()
    }

    pub fn step(
        &mut self,
        input: &str,
        end_of_input: bool,
        control: &mut WorkControl,
    ) -> Result<(usize, WorkPoll<String>), BuiltinError> {
        let mut consumed = 0usize;
        loop {
            while let Some(ch) = self.pending.get(self.pending_index).copied() {
                if !self.output.append_char(ch, control)? {
                    return Ok((consumed, WorkPoll::Pending));
                }
                self.pending_index += 1;
            }
            self.pending.clear();
            self.pending_index = 0;
            let Some(ch) = input[consumed..].chars().next() else {
                if end_of_input {
                    self.resolve_sigma(false);
                    return Ok((consumed, WorkPoll::Ready(self.output.take())));
                }
                return Ok((consumed, WorkPoll::Pending));
            };
            if self.mapping == CaseMapping::Lowercase && !unicode::is_case_ignorable(ch) {
                let following_is_cased = unicode::is_cased(ch);
                self.resolve_sigma(following_is_cased);
                if ch == 'Σ' && self.preceded_by_cased {
                    self.pending_sigma = Some(self.output.len());
                }
                self.preceded_by_cased = following_is_cased;
            }
            self.pending.extend(self.mapping.scalar(ch));
            consumed += ch.len_utf8();
        }
    }

    fn resolve_sigma(&mut self, followed_by_cased: bool) {
        if let Some(offset) = self.pending_sigma.take()
            && !followed_by_cased
        {
            self.output.replace_same_width(offset, "σ", "ς");
        }
    }
}

fn lowercase(chars: impl Iterator<Item = char>, output: &mut String) {
    let mut preceded_by_cased = false;
    let mut pending_sigma = None;
    for ch in chars {
        if !unicode::is_case_ignorable(ch) {
            let following_is_cased = unicode::is_cased(ch);
            resolve_sigma(output, &mut pending_sigma, following_is_cased);
            if ch == 'Σ' && preceded_by_cased {
                pending_sigma = Some(output.len());
            }
            preceded_by_cased = following_is_cased;
        }
        output.extend(CaseMapping::Lowercase.scalar(ch));
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
