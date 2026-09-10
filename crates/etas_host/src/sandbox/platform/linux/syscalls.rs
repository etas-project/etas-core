use std::io;

#[cfg(test)]
#[path = "syscall_tests.rs"]
mod tests;

const LOAD: u16 = (libc::BPF_LD | libc::BPF_W | libc::BPF_ABS) as u16;
const EQ: u16 = (libc::BPF_JMP | libc::BPF_JEQ | libc::BPF_K) as u16;
const AND: u16 = (libc::BPF_ALU | libc::BPF_AND | libc::BPF_K) as u16;
const RET: u16 = (libc::BPF_RET | libc::BPF_K) as u16;
const ALLOW: u32 = libc::SECCOMP_RET_ALLOW;
const DENY: u32 = libc::SECCOMP_RET_ERRNO | libc::EPERM as u32;
#[cfg(target_arch = "aarch64")]
const ARCH: u32 = 0xc00000b7;
#[cfg(target_arch = "x86_64")]
const ARCH: u32 = 0xc000003e;

pub(super) struct Filter {
    code: Vec<libc::sock_filter>,
    own_pid_checks: Vec<usize>,
}

impl Filter {
    pub(super) fn new() -> Self {
        let mut filter = Self {
            code: vec![
                insn(LOAD, 4),
                jump(ARCH, 1, 0),
                insn(RET, libc::SECCOMP_RET_KILL_PROCESS),
                insn(LOAD, 0),
            ],
            own_pid_checks: Vec::new(),
        };
        // clone3's pointer arguments cannot be inspected by classic BPF. ENOSYS
        // lets libc fall back to the explicitly checked clone ABI.
        filter.code.extend([
            jump(libc::SYS_clone3 as u32, 0, 1),
            insn(RET, libc::SECCOMP_RET_ERRNO | libc::ENOSYS as u32),
        ]);
        filter.thread_clone();
        for syscall in [libc::SYS_tgkill, libc::SYS_kill] {
            filter.own_process(syscall, false);
        }
        for syscall in [libc::SYS_prlimit64, libc::SYS_sched_getaffinity] {
            filter.own_process(syscall, true);
        }
        filter.fcntl();
        // Pipe nonblocking/readiness controls are needed by subprocess I/O.
        // No terminal injection, device control or interface ioctls are exposed.
        filter.code.extend([
            jump(libc::SYS_ioctl as u32, 0, 6),
            insn(LOAD, 24),
            jump(libc::FIONBIO as u32, 0, 1),
            insn(RET, ALLOW),
            jump(libc::FIONREAD as u32, 0, 1),
            insn(RET, ALLOW),
            insn(RET, DENY),
        ]);
        // Default deny: no sockets (including UDP/UNIX), ptrace, process_vm,
        // pidfd, mount, namespace, io_uring, bpf, signal-to-peer or metadata writes.
        // Filesystem data operations below are additionally mediated by Landlock.
        for syscall in allowed_syscalls() {
            filter
                .code
                .extend([jump(syscall as u32, 0, 1), insn(RET, ALLOW)]);
        }
        filter.code.push(insn(RET, DENY));
        filter
    }

    fn own_process(&mut self, syscall: libc::c_long, allow_zero: bool) {
        // seccomp_data.args[0] is a 64-bit value at offset 16. pid_t is 32-bit;
        // matching its effective low word cannot authorize another process.
        let mut block = vec![insn(LOAD, 16)];
        if allow_zero {
            block.extend([jump(0, 0, 1), insn(RET, ALLOW)]);
        }
        let pid_index = self.code.len() + 1 + block.len();
        block.extend([jump(0, 0, 1), insn(RET, ALLOW), insn(RET, DENY)]);
        self.code.push(jump(syscall as u32, 0, block.len() as u8));
        self.code.extend(block);
        self.own_pid_checks.push(pid_index);
    }

    fn thread_clone(&mut self) {
        let required = (libc::CLONE_THREAD | libc::CLONE_VM | libc::CLONE_SIGHAND) as u32;
        let allowed = required
            | (libc::CLONE_FS
                | libc::CLONE_FILES
                | libc::CLONE_SYSVSEM
                | libc::CLONE_SETTLS
                | libc::CLONE_PARENT_SETTID
                | libc::CLONE_CHILD_CLEARTID
                | libc::CLONE_CHILD_SETTID) as u32;
        // Process children may fork/vfork but cannot select a new parent,
        // namespace, or process group. They inherit both kernel sandboxes.
        let process_allowed = allowed | libc::CLONE_VFORK as u32 | libc::SIGCHLD as u32;
        let block = [
            insn(LOAD, 20),
            jump(0, 1, 0),
            insn(RET, DENY),
            insn(LOAD, 16),
            insn(AND, 0xff),
            jump(libc::SIGCHLD as u32, 0, 6),
            insn(LOAD, 16),
            insn(AND, !process_allowed),
            jump(0, 0, 1),
            insn(RET, ALLOW),
            insn(RET, DENY),
            insn(RET, DENY),
            insn(LOAD, 16),
            insn(AND, !allowed),
            jump(0, 1, 0),
            insn(RET, DENY),
            insn(LOAD, 16),
            insn(AND, required),
            jump(required, 0, 1),
            insn(RET, ALLOW),
            insn(RET, DENY),
        ];
        self.code
            .push(jump(libc::SYS_clone as u32, 0, block.len() as u8));
        self.code.extend(block);
    }

    fn fcntl(&mut self) {
        let mut block = vec![insn(LOAD, 24)];
        for command in [
            libc::F_DUPFD,
            libc::F_DUPFD_CLOEXEC,
            libc::F_GETFD,
            libc::F_SETFD,
            libc::F_GETFL,
            libc::F_SETFL,
            libc::F_GETLK,
            libc::F_SETLK,
            libc::F_SETLKW,
            libc::F_OFD_GETLK,
            libc::F_OFD_SETLK,
            libc::F_OFD_SETLKW,
        ] {
            block.extend([jump(command as u32, 0, 1), insn(RET, ALLOW)]);
        }
        block.push(insn(RET, DENY));
        self.code
            .push(jump(libc::SYS_fcntl as u32, 0, block.len() as u8));
        self.code.extend(block);
    }

    pub(super) fn install(&mut self) -> io::Result<()> {
        let pid = unsafe { libc::getpid() } as u32;
        for index in &self.own_pid_checks {
            self.code[*index].k = pid;
        }
        let program = libc::sock_fprog {
            len: self.code.len() as u16,
            filter: self.code.as_mut_ptr(),
        };
        if unsafe { libc::prctl(libc::PR_SET_SECCOMP, libc::SECCOMP_MODE_FILTER, &program) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

fn insn(code: u16, k: u32) -> libc::sock_filter {
    libc::sock_filter {
        code,
        jt: 0,
        jf: 0,
        k,
    }
}
fn jump(k: u32, jt: u8, jf: u8) -> libc::sock_filter {
    libc::sock_filter {
        code: EQ,
        jt,
        jf,
        k,
    }
}

fn allowed_syscalls() -> Vec<libc::c_long> {
    let calls = vec![
        libc::SYS_read,
        libc::SYS_write,
        libc::SYS_readv,
        libc::SYS_writev,
        libc::SYS_close,
        libc::SYS_close_range,
        libc::SYS_lseek,
        libc::SYS_pread64,
        libc::SYS_pwrite64,
        libc::SYS_preadv,
        libc::SYS_pwritev,
        libc::SYS_preadv2,
        libc::SYS_pwritev2,
        libc::SYS_openat,
        libc::SYS_openat2,
        libc::SYS_fstat,
        libc::SYS_newfstatat,
        libc::SYS_statx,
        libc::SYS_getdents64,
        libc::SYS_readlinkat,
        libc::SYS_faccessat,
        libc::SYS_faccessat2,
        libc::SYS_mkdirat,
        libc::SYS_unlinkat,
        libc::SYS_renameat,
        libc::SYS_renameat2,
        libc::SYS_linkat,
        libc::SYS_symlinkat,
        libc::SYS_fsync,
        libc::SYS_fdatasync,
        libc::SYS_truncate,
        libc::SYS_ftruncate,
        libc::SYS_dup,
        libc::SYS_dup3,
        libc::SYS_flock,
        libc::SYS_getcwd,
        libc::SYS_chdir,
        libc::SYS_fchdir,
        libc::SYS_umask,
        libc::SYS_mmap,
        libc::SYS_mprotect,
        libc::SYS_munmap,
        libc::SYS_mremap,
        libc::SYS_madvise,
        libc::SYS_brk,
        libc::SYS_rt_sigaction,
        libc::SYS_rt_sigprocmask,
        libc::SYS_rt_sigreturn,
        libc::SYS_rt_sigpending,
        libc::SYS_rt_sigtimedwait,
        libc::SYS_rt_sigsuspend,
        libc::SYS_sigaltstack,
        libc::SYS_getpid,
        libc::SYS_getppid,
        libc::SYS_gettid,
        libc::SYS_getuid,
        libc::SYS_geteuid,
        libc::SYS_getgid,
        libc::SYS_getegid,
        libc::SYS_getgroups,
        libc::SYS_exit,
        libc::SYS_exit_group,
        libc::SYS_execve,
        libc::SYS_execveat,
        libc::SYS_wait4,
        libc::SYS_waitid,
        libc::SYS_futex,
        libc::SYS_futex_waitv,
        libc::SYS_set_robust_list,
        libc::SYS_set_tid_address,
        libc::SYS_rseq,
        libc::SYS_getrandom,
        libc::SYS_clock_gettime,
        libc::SYS_clock_getres,
        libc::SYS_clock_nanosleep,
        libc::SYS_nanosleep,
        libc::SYS_gettimeofday,
        libc::SYS_times,
        libc::SYS_getrusage,
        libc::SYS_uname,
        libc::SYS_sched_yield,
        libc::SYS_getrlimit,
        libc::SYS_setrlimit,
        libc::SYS_ppoll,
        libc::SYS_pselect6,
        libc::SYS_epoll_create1,
        libc::SYS_epoll_ctl,
        libc::SYS_epoll_pwait,
        libc::SYS_epoll_pwait2,
        libc::SYS_eventfd2,
        libc::SYS_pipe2,
        libc::SYS_restart_syscall,
    ];
    #[cfg(target_arch = "x86_64")]
    let calls = {
        let mut calls = calls;
        calls.extend([
            libc::SYS_open,
            libc::SYS_stat,
            libc::SYS_lstat,
            libc::SYS_readlink,
            libc::SYS_access,
            libc::SYS_mkdir,
            libc::SYS_rmdir,
            libc::SYS_unlink,
            libc::SYS_rename,
            libc::SYS_link,
            libc::SYS_symlink,
            libc::SYS_pipe,
            libc::SYS_dup2,
            libc::SYS_poll,
            libc::SYS_select,
            libc::SYS_arch_prctl,
            libc::SYS_epoll_wait,
            libc::SYS_fork,
            libc::SYS_vfork,
        ]);
        calls
    };
    calls
}
