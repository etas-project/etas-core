pub fn i64_to_i32(value: i64) -> Option<i32> {
    i32::try_from(value).ok()
}
