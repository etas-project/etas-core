use std::cmp::Ordering;

pub fn compare_i64(lhs: i64, rhs: i64) -> Ordering {
    lhs.cmp(&rhs)
}
