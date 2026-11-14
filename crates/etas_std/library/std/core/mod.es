module std.core;

// Terminating execution is a runtime capability; callers own their error policy.
@std.binding("std.core.abort")
public flow abort(message: string) -> never ![] {}

@std.role("std.core.Index")
public spec Index;
impl i8 ~ Index;
impl i16 ~ Index;
impl i32 ~ Index;
impl i64 ~ Index;
impl i128 ~ Index;
impl isize ~ Index;
impl u8 ~ Index;
impl u16 ~ Index;
impl u32 ~ Index;
impl u64 ~ Index;
impl u128 ~ Index;
impl usize ~ Index;
