use std::time::Duration;

use crate::HostError;

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub(super) fn sample() -> Result<(Duration, u64), HostError> {
    use std::mem::MaybeUninit;

    let mut clock = MaybeUninit::<libc::timespec>::uninit();
    // SAFETY: clock points to writable storage; it is read only after success.
    if unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, clock.as_mut_ptr()) } != 0 {
        return Err(super::unavailable(format!(
            "cannot read compiler worker CPU clock: {}",
            std::io::Error::last_os_error()
        )));
    }
    // SAFETY: a successful clock_gettime initializes the timespec.
    let clock = unsafe { clock.assume_init() };
    let seconds = u64::try_from(clock.tv_sec)
        .map_err(|_| super::unavailable("negative compiler worker CPU time"))?;
    let nanos = u32::try_from(clock.tv_nsec)
        .ok()
        .filter(|value| *value < 1_000_000_000)
        .ok_or_else(|| super::unavailable("invalid compiler worker CPU nanoseconds"))?;

    let mut usage = MaybeUninit::<libc::rusage>::zeroed();
    // SAFETY: usage is initialized writable storage for the process counters.
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) } != 0 {
        return Err(super::unavailable(format!(
            "cannot read compiler process resident memory: {}",
            std::io::Error::last_os_error()
        )));
    }
    // SAFETY: the storage was zeroed and getrusage succeeded.
    let usage = unsafe { usage.assume_init() };
    let resident = u64::try_from(usage.ru_maxrss)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| super::unavailable("invalid compiler process peak resident memory"))?;
    // Darwin reports bytes; Linux reports KiB. Keep the public contract in bytes.
    #[cfg(target_os = "linux")]
    let resident = resident
        .checked_mul(1024)
        .ok_or_else(|| super::unavailable("compiler resident-memory counter overflow"))?;
    Ok((Duration::new(seconds, nanos), resident))
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub(super) fn sample() -> Result<(Duration, u64), HostError> {
    Err(super::unavailable(
        "native compiler resource measurement is unavailable on this platform",
    ))
}
