use std::borrow::Cow;

use etas_std::{StdIntrinsicId, intrinsic::pure};

use crate::{BuiltinError, BuiltinValue};

use super::{
    case::{CaseMapping, CaseOutput, map_case},
    ranges::{TextTrimKind, trim_range_kind},
};

#[derive(Clone, Copy, Debug)]
pub enum TextTransform {
    Trim,
    Lowercase,
    Uppercase,
    Split,
}

pub enum TextOutput<'a> {
    String(Cow<'a, str>),
    Array(TextParts<'a>),
}

pub enum TextParts<'a> {
    Split(std::str::Split<'a, &'a str>),
}

impl<'a> Iterator for TextParts<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Split(parts) => parts.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Split(parts) => parts.size_hint(),
        }
    }
}

impl TextOutput<'_> {
    pub fn into_owned(self) -> BuiltinValue {
        match self {
            Self::String(text) => BuiltinValue::String(text.into_owned()),
            Self::Array(parts) => BuiltinValue::Array(
                parts
                    .map(|part| BuiltinValue::String(part.to_owned()))
                    .collect(),
            ),
        }
    }
}

impl TextTransform {
    pub fn for_intrinsic(id: StdIntrinsicId) -> Option<Self> {
        match id.0 {
            pure::TEXT_TRIM => Some(Self::Trim),
            pure::TEXT_LOWERCASE => Some(Self::Lowercase),
            pure::TEXT_UPPERCASE => Some(Self::Uppercase),
            pure::TEXT_SPLIT => Some(Self::Split),
            _ => None,
        }
    }

    pub fn arity(self) -> usize {
        match self {
            Self::Split => 2,
            _ => 1,
        }
    }

    pub fn evaluate<'a>(self, args: &[&'a str]) -> Result<TextOutput<'a>, BuiltinError> {
        if args.len() != self.arity() {
            return Err(BuiltinError::ArityMismatch {
                expected: self.arity(),
                actual: args.len(),
            });
        }
        Ok(match self {
            Self::Trim => TextOutput::String(Cow::Borrowed(
                &args[0][trim_range_kind(args[0], TextTrimKind::Both)],
            )),
            Self::Lowercase | Self::Uppercase => {
                let mapping = if matches!(self, Self::Lowercase) {
                    CaseMapping::Lowercase
                } else {
                    CaseMapping::Uppercase
                };
                match map_case(args[0], mapping)? {
                    CaseOutput::Unchanged => TextOutput::String(Cow::Borrowed(args[0])),
                    CaseOutput::Changed(output) => TextOutput::String(Cow::Owned(output)),
                }
            }
            Self::Split => TextOutput::Array(TextParts::Split(args[0].split(args[1]))),
        })
    }
}

#[cfg(test)]
mod tests;
