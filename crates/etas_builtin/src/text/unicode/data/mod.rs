#[derive(Clone, Copy)]
pub(super) struct DecompositionRecord {
    pub scalar: u32,
    pub compatibility: bool,
    pub offset: u32,
    pub len: u8,
}

#[derive(Clone, Copy)]
pub(super) struct MappingRecord {
    pub scalar: u32,
    pub offset: u32,
    pub len: u8,
}

include!("generated.rs");

pub(super) fn combining_class(scalar: u32) -> u8 {
    COMBINING_CLASSES
        .binary_search_by_key(&scalar, |(scalar, _)| *scalar)
        .map_or(0, |index| COMBINING_CLASSES[index].1)
}

pub(super) fn decomposition(scalar: u32, compatibility: bool) -> Option<&'static [u32]> {
    let index = DECOMPOSITIONS
        .binary_search_by_key(&scalar, |record| record.scalar)
        .ok()?;
    let record = DECOMPOSITIONS[index];
    if record.compatibility && !compatibility {
        return None;
    }
    let start = record.offset as usize;
    Some(&DECOMPOSITION_SCALARS[start..start + usize::from(record.len)])
}

pub(super) fn composition(starter: u32, combining: u32) -> Option<u32> {
    let key = (u64::from(starter) << 21) | u64::from(combining);
    COMPOSITIONS
        .binary_search_by_key(&key, |(key, _)| *key)
        .ok()
        .map(|index| COMPOSITIONS[index].1)
}

pub(super) fn casefold(scalar: u32) -> Option<&'static [u32]> {
    let index = CASEFOLDS
        .binary_search_by_key(&scalar, |record| record.scalar)
        .ok()?;
    let record = CASEFOLDS[index];
    let start = record.offset as usize;
    Some(&CASEFOLD_SCALARS[start..start + usize::from(record.len)])
}

fn contains(ranges: &[(u32, u32)], scalar: u32) -> bool {
    ranges
        .binary_search_by(|(start, end)| {
            if *end < scalar {
                std::cmp::Ordering::Less
            } else if *start > scalar {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

pub(super) fn is_whitespace(scalar: u32) -> bool {
    contains(WHITE_SPACE_RANGES, scalar)
}

pub(super) fn is_alphabetic(scalar: u32) -> bool {
    contains(ALPHABETIC_RANGES, scalar)
}

pub(super) fn is_cased(scalar: u32) -> bool {
    contains(CASED_RANGES, scalar)
}

pub(super) fn is_case_ignorable(scalar: u32) -> bool {
    contains(CASE_IGNORABLE_RANGES, scalar)
}

pub(super) fn is_numeric(scalar: u32) -> bool {
    contains(NUMERIC_RANGES, scalar)
}

pub(super) fn is_control(scalar: u32) -> bool {
    contains(CONTROL_RANGES, scalar)
}

fn mapping(
    mappings: &'static [MappingRecord],
    scalars: &'static [u32],
    scalar: u32,
) -> Option<&'static [u32]> {
    let index = mappings
        .binary_search_by_key(&scalar, |record| record.scalar)
        .ok()?;
    let record = mappings[index];
    let start = record.offset as usize;
    Some(&scalars[start..start + usize::from(record.len)])
}

pub(super) fn lowercase(scalar: u32) -> Option<&'static [u32]> {
    mapping(LOWERCASE_MAPPINGS, LOWERCASE_SCALARS, scalar)
}

pub(super) fn uppercase(scalar: u32) -> Option<&'static [u32]> {
    mapping(UPPERCASE_MAPPINGS, UPPERCASE_SCALARS, scalar)
}
