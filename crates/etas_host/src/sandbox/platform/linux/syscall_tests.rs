use super::Filter;
use std::{io, os::unix::process::CommandExt, process::Command};

const CHILD: &str = "sandbox::platform::linux::syscalls::tests::seccomp_child_probe";

#[test]
fn seccomp_enforces_process_and_network_rules_across_exec_and_descendants() {
    let mut filter = Filter::new();
    let mut child = Command::new(std::env::current_exe().unwrap());
    child.args(["--ignored", "--exact", CHILD, "--nocapture"]);
    child.env("ETAS_SECCOMP_PROBE", "parent");
    child.env("ETAS_SECCOMP_PARENT", std::process::id().to_string());
    unsafe {
        child.pre_exec(move || {
            if libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0 {
                return Err(io::Error::last_os_error());
            }
            filter.install()
        });
    }
    let output = child.output().unwrap();
    assert!(output.status.success(), "{output:?}");
}

#[test]
#[ignore = "launched in a subprocess after installing the actual kernel filter"]
fn seccomp_child_probe() {
    let mode = std::env::var("ETAS_SECCOMP_PROBE").unwrap();
    assert!(matches!(mode.as_str(), "parent" | "descendant"));
    assert_eq!(
        std::net::UdpSocket::bind("127.0.0.1:0")
            .unwrap_err()
            .raw_os_error(),
        Some(libc::EPERM)
    );
    assert_eq!(
        std::os::unix::net::UnixStream::pair()
            .unwrap_err()
            .raw_os_error(),
        Some(libc::EPERM)
    );
    let parent = std::env::var("ETAS_SECCOMP_PARENT")
        .unwrap()
        .parse::<i32>()
        .unwrap();
    assert_eq!(unsafe { libc::kill(parent, 0) }, -1);
    assert_eq!(io::Error::last_os_error().raw_os_error(), Some(libc::EPERM));
    assert_eq!(unsafe { libc::setsid() }, -1);
    assert_eq!(unsafe { libc::setpgid(0, 0) }, -1);
    assert_eq!(unsafe { libc::unshare(0) }, -1);
    assert_eq!(std::thread::spawn(|| 42).join().unwrap(), 42);
    if mode == "parent" {
        let output = Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", CHILD, "--nocapture"])
            .env("ETAS_SECCOMP_PROBE", "descendant")
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
    }
}
