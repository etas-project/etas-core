use etas_std::{StdIntrinsicId, intrinsic::pure};

use crate::{
    BuiltinValue, TextFailure,
    work::{WorkControl, WorkPoll, WorkStopReason},
};

use super::input::TextInput;

pub fn error_value(error: TextFailure) -> BuiltinValue {
    BuiltinValue::Variant {
        name: error.variant_name().into(),
        fields: Vec::new(),
    }
}

pub fn result(value: Result<BuiltinValue, TextFailure>) -> BuiltinValue {
    match value {
        Ok(value) => BuiltinValue::ResultOk(Box::new(value)),
        Err(error) => BuiltinValue::ResultErr(Box::new(error_value(error))),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParsedInteger {
    negative: bool,
    magnitude: u128,
}

#[derive(Clone, Debug, Default)]
pub struct TextIntegerParseWork {
    started: bool,
    negative: bool,
    magnitude: u128,
    digits: usize,
    overflow: bool,
}

impl TextIntegerParseWork {
    pub fn step(
        &mut self,
        input: &str,
        end_of_input: bool,
        control: &mut WorkControl,
    ) -> (usize, WorkPoll<Result<ParsedInteger, TextFailure>>) {
        let mut consumed = 0usize;
        for byte in input.bytes() {
            if control.consume(1) == Err(WorkStopReason::Yield) {
                return (consumed, WorkPoll::Pending);
            }
            consumed += 1;
            if !self.started {
                self.started = true;
                match byte {
                    b'-' => {
                        self.negative = true;
                        continue;
                    }
                    b'+' => continue,
                    _ => {}
                }
            }
            if !byte.is_ascii_digit() {
                return (consumed, WorkPoll::Ready(Err(TextFailure::ParseFailure)));
            }
            self.digits += 1;
            if !self.overflow {
                let digit = u128::from(byte - b'0');
                match self
                    .magnitude
                    .checked_mul(10)
                    .and_then(|value| value.checked_add(digit))
                {
                    Some(value) => self.magnitude = value,
                    None => self.overflow = true,
                }
            }
        }
        if !end_of_input || consumed != input.len() {
            return (consumed, WorkPoll::Pending);
        }
        let parsed = if self.digits == 0 {
            Err(TextFailure::ParseFailure)
        } else if self.overflow {
            Err(TextFailure::NumericOverflow)
        } else {
            Ok(ParsedInteger {
                negative: self.negative,
                magnitude: self.magnitude,
            })
        };
        (consumed, WorkPoll::Ready(parsed))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextIntegerKind {
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
}

impl TextIntegerKind {
    pub fn for_intrinsic(intrinsic: StdIntrinsicId) -> Option<Self> {
        Some(match intrinsic.0 {
            pure::TEXT_PARSE_I8 => Self::I8,
            pure::TEXT_PARSE_I16 => Self::I16,
            pure::TEXT_PARSE_I32 => Self::I32,
            pure::TEXT_PARSE_I64 => Self::I64,
            pure::TEXT_PARSE_I128 => Self::I128,
            pure::TEXT_PARSE_ISIZE => Self::Isize,
            pure::TEXT_PARSE_U8 => Self::U8,
            pure::TEXT_PARSE_U16 => Self::U16,
            pure::TEXT_PARSE_U32 => Self::U32,
            pure::TEXT_PARSE_U64 => Self::U64,
            pure::TEXT_PARSE_U128 => Self::U128,
            pure::TEXT_PARSE_USIZE => Self::Usize,
            _ => return None,
        })
    }

    pub fn finish(self, parsed: ParsedInteger) -> Result<BuiltinValue, TextFailure> {
        macro_rules! signed {
            ($ty:ty, $variant:ident) => {
                parse_signed(parsed, <$ty>::MAX as u128)
                    .map(|value| BuiltinValue::$variant(value as $ty))
            };
        }
        macro_rules! unsigned {
            ($ty:ty, $variant:ident) => {
                parse_unsigned(parsed, <$ty>::MAX as u128)
                    .map(|value| BuiltinValue::$variant(value as $ty))
            };
        }
        match self {
            Self::I8 => signed!(i8, I8),
            Self::I16 => signed!(i16, I16),
            Self::I32 => signed!(i32, I32),
            Self::I64 => signed!(i64, I64),
            Self::I128 => signed!(i128, I128),
            Self::Isize => signed!(isize, Isize),
            Self::U8 => unsigned!(u8, U8),
            Self::U16 => unsigned!(u16, U16),
            Self::U32 => unsigned!(u32, U32),
            Self::U64 => unsigned!(u64, U64),
            Self::U128 => unsigned!(u128, U128),
            Self::Usize => unsigned!(usize, Usize),
        }
    }
}

fn parse_integer_magnitude<T: TextInput + ?Sized>(text: &T) -> Result<ParsedInteger, TextFailure> {
    let mut work = TextIntegerParseWork::default();
    let mut control = WorkControl::new(usize::MAX, 0, 0);
    let mut chunks = text.chunks().peekable();
    while let Some(chunk) = chunks.next() {
        let (consumed, poll) = work.step(chunk, chunks.peek().is_none(), &mut control);
        debug_assert_eq!(consumed, chunk.len());
        if let WorkPoll::Ready(parsed) = poll {
            return parsed;
        }
    }
    let (_, WorkPoll::Ready(parsed)) = work.step("", true, &mut control) else {
        unreachable!("empty integer parse must complete")
    };
    parsed
}

fn parse_signed(parsed: ParsedInteger, max: u128) -> Result<i128, TextFailure> {
    let ParsedInteger {
        negative,
        magnitude,
    } = parsed;
    let negative_limit = max + 1;
    if (!negative && magnitude > max) || (negative && magnitude > negative_limit) {
        return Err(TextFailure::NumericOverflow);
    }
    if negative {
        if magnitude == (i128::MAX as u128) + 1 {
            Ok(i128::MIN)
        } else {
            Ok(-(magnitude as i128))
        }
    } else {
        Ok(magnitude as i128)
    }
}

fn parse_unsigned(parsed: ParsedInteger, max: u128) -> Result<u128, TextFailure> {
    let ParsedInteger {
        negative,
        magnitude,
    } = parsed;
    if negative {
        return Err(TextFailure::ParseFailure);
    }
    if magnitude > max {
        return Err(TextFailure::NumericOverflow);
    }
    Ok(magnitude)
}

macro_rules! parse_unsigned_integer {
    ($name:ident, $input_name:ident, $ty:ty, $variant:ident) => {
        pub fn $name(text: &str) -> Result<BuiltinValue, TextFailure> {
            $input_name(text)
        }

        pub fn $input_name<T: TextInput + ?Sized>(text: &T) -> Result<BuiltinValue, TextFailure> {
            TextIntegerKind::$variant.finish(parse_integer_magnitude(text)?)
        }
    };
}

macro_rules! parse_signed_integer_named {
    ($name:ident, $input_name:ident, $ty:ty, $variant:ident) => {
        pub fn $name(text: &str) -> Result<BuiltinValue, TextFailure> {
            $input_name(text)
        }

        pub fn $input_name<T: TextInput + ?Sized>(text: &T) -> Result<BuiltinValue, TextFailure> {
            TextIntegerKind::$variant.finish(parse_integer_magnitude(text)?)
        }
    };
}

parse_signed_integer_named!(parse_i8, parse_i8_input, i8, I8);
parse_signed_integer_named!(parse_i16, parse_i16_input, i16, I16);
parse_signed_integer_named!(parse_i32, parse_i32_input, i32, I32);
parse_signed_integer_named!(parse_i64, parse_i64_input, i64, I64);
parse_signed_integer_named!(parse_i128, parse_i128_input, i128, I128);
parse_signed_integer_named!(parse_isize, parse_isize_input, isize, Isize);
parse_unsigned_integer!(parse_u8, parse_u8_input, u8, U8);
parse_unsigned_integer!(parse_u16, parse_u16_input, u16, U16);
parse_unsigned_integer!(parse_u32, parse_u32_input, u32, U32);
parse_unsigned_integer!(parse_u64, parse_u64_input, u64, U64);
parse_unsigned_integer!(parse_u128, parse_u128_input, u128, U128);
parse_unsigned_integer!(parse_usize, parse_usize_input, usize, Usize);

const MAX_FLOAT_SIGNIFICAND_DIGITS: usize = 768;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedFloat {
    negative: bool,
    significand: Vec<u8>,
    decimal_exponent: i128,
}

impl ParsedFloat {
    fn canonical(&self) -> String {
        if self.significand.is_empty() {
            return if self.negative { "-0" } else { "0" }.to_owned();
        }
        let mut output = String::with_capacity(self.significand.len() + 48);
        if self.negative {
            output.push('-');
        }
        output.push(char::from(self.significand[0]));
        if self.significand.len() > 1 {
            output.push('.');
            output.extend(self.significand[1..].iter().copied().map(char::from));
        }
        output.push('e');
        output.push_str(&self.decimal_exponent.to_string());
        output
    }
}

#[derive(Clone, Debug, Default)]
pub struct TextFloatParseWork {
    position: usize,
    negative: bool,
    mantissa_digits: usize,
    digits_before_decimal: usize,
    first_nonzero_digit: Option<usize>,
    significand: Vec<u8>,
    saw_decimal_point: bool,
    saw_exponent: bool,
    exponent_chars: usize,
    exponent_negative: bool,
    exponent_digits: usize,
    exponent_magnitude: u128,
}

impl TextFloatParseWork {
    pub fn step(
        &mut self,
        input: &str,
        end_of_input: bool,
        control: &mut WorkControl,
    ) -> (usize, WorkPoll<Result<ParsedFloat, TextFailure>>) {
        let mut consumed = 0usize;
        for byte in input.bytes() {
            if control.consume(1) == Err(WorkStopReason::Yield) {
                return (consumed, WorkPoll::Pending);
            }
            consumed += 1;
            if self.position == 0 && matches!(byte, b'+' | b'-') {
                self.negative = byte == b'-';
                self.position += 1;
                continue;
            }
            self.position += 1;
            if self.saw_exponent {
                if self.exponent_chars == 0 && matches!(byte, b'+' | b'-') {
                    self.exponent_negative = byte == b'-';
                    self.exponent_chars += 1;
                    continue;
                }
                self.exponent_chars += 1;
                if !byte.is_ascii_digit() {
                    return (consumed, WorkPoll::Ready(Err(TextFailure::ParseFailure)));
                }
                self.exponent_digits += 1;
                self.exponent_magnitude = self
                    .exponent_magnitude
                    .saturating_mul(10)
                    .saturating_add(u128::from(byte - b'0'));
                continue;
            }
            match byte {
                b'0'..=b'9' => {
                    let digit_index = self.mantissa_digits;
                    self.mantissa_digits = self.mantissa_digits.saturating_add(1);
                    if !self.saw_decimal_point {
                        self.digits_before_decimal = self.digits_before_decimal.saturating_add(1);
                    }
                    if self.first_nonzero_digit.is_none() && byte != b'0' {
                        self.first_nonzero_digit = Some(digit_index);
                    }
                    if self.first_nonzero_digit.is_some()
                        && self.significand.len() < MAX_FLOAT_SIGNIFICAND_DIGITS
                    {
                        self.significand.push(byte);
                    }
                }
                b'.' if !self.saw_decimal_point => self.saw_decimal_point = true,
                b'e' | b'E' if self.mantissa_digits > 0 => self.saw_exponent = true,
                _ => return (consumed, WorkPoll::Ready(Err(TextFailure::ParseFailure))),
            }
        }
        if !end_of_input || consumed != input.len() {
            return (consumed, WorkPoll::Pending);
        }
        if self.mantissa_digits == 0 || (self.saw_exponent && self.exponent_digits == 0) {
            return (consumed, WorkPoll::Ready(Err(TextFailure::ParseFailure)));
        }
        let Some(first_nonzero) = self.first_nonzero_digit else {
            return (
                consumed,
                WorkPoll::Ready(Ok(ParsedFloat {
                    negative: self.negative,
                    significand: Vec::new(),
                    decimal_exponent: 0,
                })),
            );
        };
        let before_decimal = i128::try_from(self.digits_before_decimal).unwrap_or(i128::MAX);
        let first_nonzero = i128::try_from(first_nonzero).unwrap_or(i128::MAX);
        let exponent_magnitude = i128::try_from(self.exponent_magnitude).unwrap_or(i128::MAX);
        let explicit_exponent = if self.exponent_negative {
            exponent_magnitude.saturating_neg()
        } else {
            exponent_magnitude
        };
        let decimal_exponent = before_decimal
            .saturating_sub(first_nonzero)
            .saturating_sub(1)
            .saturating_add(explicit_exponent);
        (
            consumed,
            WorkPoll::Ready(Ok(ParsedFloat {
                negative: self.negative,
                significand: std::mem::take(&mut self.significand),
                decimal_exponent,
            })),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextFloatKind {
    F32,
    F64,
}

impl TextFloatKind {
    pub fn for_intrinsic(intrinsic: StdIntrinsicId) -> Option<Self> {
        match intrinsic.0 {
            pure::TEXT_PARSE_F32 => Some(Self::F32),
            pure::TEXT_PARSE_F64 => Some(Self::F64),
            _ => None,
        }
    }

    pub fn finish(self, parsed: ParsedFloat) -> Result<BuiltinValue, TextFailure> {
        let nonzero = !parsed.significand.is_empty();
        let canonical = parsed.canonical();
        match self {
            Self::F32 => {
                let value = canonical
                    .parse::<f32>()
                    .map_err(|_| TextFailure::ParseFailure)?;
                validate_float(nonzero, value.is_finite(), value == 0.0)?;
                Ok(BuiltinValue::F32(value))
            }
            Self::F64 => {
                let value = canonical
                    .parse::<f64>()
                    .map_err(|_| TextFailure::ParseFailure)?;
                validate_float(nonzero, value.is_finite(), value == 0.0)?;
                Ok(BuiltinValue::F64(value))
            }
        }
    }
}

fn parse_float_input<T: TextInput + ?Sized>(
    text: &T,
    kind: TextFloatKind,
) -> Result<BuiltinValue, TextFailure> {
    let mut work = TextFloatParseWork::default();
    let mut control = WorkControl::new(usize::MAX, 0, 0);
    let mut chunks = text.chunks().peekable();
    while let Some(chunk) = chunks.next() {
        let (consumed, poll) = work.step(chunk, chunks.peek().is_none(), &mut control);
        match poll {
            WorkPoll::Ready(parsed) => return kind.finish(parsed?),
            WorkPoll::Pending => debug_assert_eq!(consumed, chunk.len()),
        }
    }
    let (_, WorkPoll::Ready(parsed)) = work.step("", true, &mut control) else {
        unreachable!("empty float parse must complete")
    };
    kind.finish(parsed?)
}

pub fn parse_f32(text: &str) -> Result<BuiltinValue, TextFailure> {
    parse_float_input(text, TextFloatKind::F32)
}

pub fn parse_f64(text: &str) -> Result<BuiltinValue, TextFailure> {
    parse_float_input(text, TextFloatKind::F64)
}

fn validate_float(nonzero: bool, finite: bool, zero: bool) -> Result<(), TextFailure> {
    if !finite || (zero && nonzero) {
        return Err(TextFailure::NumericOverflow);
    }
    Ok(())
}

pub fn parse_bool(text: &str) -> Result<BuiltinValue, TextFailure> {
    match text {
        "true" => Ok(BuiltinValue::Bool(true)),
        "false" => Ok(BuiltinValue::Bool(false)),
        _ => Err(TextFailure::ParseFailure),
    }
}

pub fn render(value: &BuiltinValue) -> Option<String> {
    Some(match value {
        BuiltinValue::I8(value) => value.to_string(),
        BuiltinValue::I16(value) => value.to_string(),
        BuiltinValue::I32(value) => value.to_string(),
        BuiltinValue::I64(value) => value.to_string(),
        BuiltinValue::I128(value) => value.to_string(),
        BuiltinValue::Isize(value) => value.to_string(),
        BuiltinValue::U8(value) => value.to_string(),
        BuiltinValue::U16(value) => value.to_string(),
        BuiltinValue::U32(value) => value.to_string(),
        BuiltinValue::U64(value) => value.to_string(),
        BuiltinValue::U128(value) => value.to_string(),
        BuiltinValue::Usize(value) => value.to_string(),
        BuiltinValue::F32(value) => value.to_string(),
        BuiltinValue::F64(value) => value.to_string(),
        BuiltinValue::Bool(value) => value.to_string(),
        BuiltinValue::Char(value) => value.to_string(),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn float_parser_matches_finite_rust_rounding_and_classifies_overflow() {
        for text in [
            "1.5",
            ".5",
            "1.",
            "-0",
            "3.4028235e38",
            "1.17549435e-38",
            "5e-324",
            "1.7976931348623157e308",
        ] {
            assert_eq!(
                parse_f64(text),
                Ok(BuiltinValue::F64(text.parse::<f64>().unwrap())),
                "{text}"
            );
        }
        assert_eq!(parse_f32("1e100"), Err(TextFailure::NumericOverflow));
        assert_eq!(parse_f64("1e10000"), Err(TextFailure::NumericOverflow));
        assert_eq!(parse_f64("1e-10000"), Err(TextFailure::NumericOverflow));
        assert_eq!(parse_f64("1.2.3"), Err(TextFailure::ParseFailure));
    }

    #[test]
    fn float_parser_bounds_each_scan_and_keeps_only_a_fixed_significand() {
        let text = format!("1.{}1", "0".repeat(4096));
        let mut work = TextFloatParseWork::default();
        let mut offset = 0usize;
        let parsed = loop {
            let mut control = WorkControl::new(17, 0, 0);
            let (consumed, poll) = work.step(&text[offset..], true, &mut control);
            assert!(consumed <= 17);
            offset += consumed;
            if let WorkPoll::Ready(parsed) = poll {
                break parsed.unwrap();
            }
        };
        assert!(parsed.significand.len() <= MAX_FLOAT_SIGNIFICAND_DIGITS);
        assert_eq!(
            TextFloatKind::F64.finish(parsed),
            Ok(BuiltinValue::F64(1.0))
        );
    }
}
