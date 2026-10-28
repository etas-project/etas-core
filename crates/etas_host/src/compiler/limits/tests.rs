use super::*;

fn limits(cpu: Duration, memory: u64) -> CompilerResourceLimits {
    CompilerResourceLimits::new(cpu, NonZeroU64::new(memory).unwrap()).unwrap()
}

#[test]
fn limits_have_explicit_units_and_reject_zero_cpu_time() {
    assert_eq!(
        CompilerResourceLimits::new(Duration::ZERO, NonZeroU64::MIN)
            .unwrap_err()
            .code,
        HostErrorCode::InvalidRequest
    );
    let limits = limits(Duration::from_millis(25), 8192);
    assert_eq!(limits.max_worker_cpu_time(), Duration::from_millis(25));
    assert_eq!(limits.max_process_peak_resident_bytes().get(), 8192);
    assert_eq!(
        limits
            .check(CompilerResourceUsage {
                worker_cpu_time: Duration::from_millis(25),
                process_peak_resident_bytes: 8192,
            })
            .unwrap_err()
            .code,
        HostErrorCode::BudgetExceeded
    );
    assert_eq!(
        limits
            .check(CompilerResourceUsage {
                worker_cpu_time: Duration::ZERO,
                process_peak_resident_bytes: 8193,
            })
            .unwrap_err()
            .code,
        HostErrorCode::BudgetExceeded
    );
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn native_cpu_meter_counts_work_instead_of_sleeping_wall_time() {
    let meter = CompilerResourceMeter::start(limits(Duration::from_secs(5), u64::MAX)).unwrap();
    let before = meter.check().unwrap();
    std::thread::sleep(Duration::from_millis(60));
    let after = meter.check().unwrap();
    assert!(after.worker_cpu_time >= before.worker_cpu_time);
    assert!(after.worker_cpu_time - before.worker_cpu_time < Duration::from_millis(30));
    assert!(after.process_peak_resident_bytes > 0);

    let meter = CompilerResourceMeter::start(limits(Duration::from_millis(2), u64::MAX)).unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let error = loop {
        for value in 0..10_000 {
            std::hint::black_box(value * value);
        }
        match meter.check() {
            Err(error) => break error,
            Ok(_) => assert!(
                std::time::Instant::now() < deadline,
                "CPU meter did not advance"
            ),
        }
    };
    assert_eq!(error.code, HostErrorCode::BudgetExceeded);
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn native_memory_meter_rejects_a_ceiling_below_existing_process_memory() {
    let error = CompilerResourceMeter::start(limits(Duration::from_secs(5), 1))
        .err()
        .expect("the running process uses more than one byte");
    assert_eq!(error.code, HostErrorCode::BudgetExceeded);
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn native_worker_meter_rejects_cross_thread_sampling() {
    let meter = CompilerResourceMeter::start(limits(Duration::from_secs(5), u64::MAX)).unwrap();
    let error = std::thread::spawn(move || meter.check())
        .join()
        .unwrap()
        .unwrap_err();
    assert_eq!(error.code, HostErrorCode::ProviderUnavailable);
}
