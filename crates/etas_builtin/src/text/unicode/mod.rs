mod casefold;
mod data;
mod normalize;
mod segment;

pub use casefold::{TextCasefoldWork, casefold};
pub use data::{DATA_FINGERPRINT, UNICODE_VERSION};
pub use normalize::{NormalizationForm, TextNormalizeWork, is_normalized, normalize};
pub use segment::{GraphemeScanStep, GraphemeScanner, next_grapheme_boundary, step_grapheme};

pub const DATA_VERSION: &str = UNICODE_VERSION;

pub fn is_whitespace(value: char) -> bool {
    data::is_whitespace(value.into())
}

pub fn is_alphabetic(value: char) -> bool {
    data::is_alphabetic(value.into())
}

pub fn is_alphanumeric(value: char) -> bool {
    let scalar = value.into();
    data::is_alphabetic(scalar) || data::is_numeric(scalar)
}

pub fn is_control(value: char) -> bool {
    data::is_control(value.into())
}

pub fn is_cased(value: char) -> bool {
    data::is_cased(value.into())
}

pub fn is_case_ignorable(value: char) -> bool {
    data::is_case_ignorable(value.into())
}

pub fn lowercase_mapping(value: char) -> Option<&'static [u32]> {
    data::lowercase(value.into())
}

pub fn uppercase_mapping(value: char) -> Option<&'static [u32]> {
    data::uppercase(value.into())
}
