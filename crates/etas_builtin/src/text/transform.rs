use std::borrow::Cow;

use etas_std::{StdIntrinsicId, intrinsic::pure};

use crate::{BuiltinError, BuiltinValue};

#[derive(Clone, Copy, Debug)]
pub enum TextTransform {
    Trim,
    Lowercase,
    Uppercase,
    Lines,
    Split,
}

pub enum TextOutput<'a> {
    String(Cow<'a, str>),
    Array(TextParts<'a>),
}

pub enum TextParts<'a> {
    Lines(std::str::Lines<'a>),
    Split(std::str::Split<'a, &'a str>),
}

impl<'a> Iterator for TextParts<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Lines(parts) => parts.next(),
            Self::Split(parts) => parts.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Lines(parts) => parts.size_hint(),
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
            pure::TEXT_LINES => Some(Self::Lines),
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
            Self::Trim => TextOutput::String(Cow::Borrowed(args[0].trim())),
            Self::Lowercase => TextOutput::String(Cow::Owned(args[0].to_lowercase())),
            Self::Uppercase => TextOutput::String(Cow::Owned(args[0].to_uppercase())),
            Self::Lines => TextOutput::Array(TextParts::Lines(args[0].lines())),
            Self::Split => TextOutput::Array(TextParts::Split(args[0].split(args[1]))),
        })
    }
}

#[cfg(test)]
mod tests;
