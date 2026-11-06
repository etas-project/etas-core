//! Stable semantic identities for structured task operations.

pub const RUN: u32 = 1_400;
pub const START: u32 = 1_401;
pub const AWAIT: u32 = 1_402;
pub const REQUEUE: u32 = 1_403;
pub const PARK: u32 = 1_404;
pub const JOIN_TUPLE: u32 = 1_405;
pub const JOIN_ARRAY: u32 = 1_406;
pub const TRY_JOIN: u32 = 1_407;
pub const COLLECT: u32 = 1_408;
pub const RACE: u32 = 1_409;
pub const MAP_CONCURRENT: u32 = 1_410;
pub const MAP_CONCURRENT_COLLECT: u32 = 1_411;
