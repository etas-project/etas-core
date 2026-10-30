use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use etas_host::TestWorkspace;

#[path = "landlock_symlink_race/observation.rs"]
mod observation;
use observation::{Observations, read_through, write_through};

struct Racer {
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<usize>>,
}

impl Racer {
    fn start(root: PathBuf, inside: PathBuf, outside: PathBuf) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let worker = thread::spawn(move || {
            if !wait_for(&root.join("ready"), &stopped) {
                return 0;
            }
            replace(&root, &outside);
            fs::write(root.join("outside-ready"), "ready").unwrap();
            if !wait_for(&root.join("outside-checked"), &stopped) {
                return 0;
            }
            let deadline = Instant::now() + Duration::from_secs(10);
            let mut swaps = 0;
            while !stopped.load(Ordering::Acquire) && Instant::now() < deadline {
                replace(&root, &inside);
                replace(&root, &outside);
                swaps += 2;
                if swaps == 2 {
                    fs::write(root.join("racing"), "ready").unwrap();
                }
            }
            swaps
        });
        Self {
            stop,
            worker: Some(worker),
        }
    }

    fn finish(mut self) -> usize {
        self.stop.store(true, Ordering::Release);
        self.worker.take().unwrap().join().unwrap()
    }
}

impl Drop for Racer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            // The normal path joins and checks the result in finish; this also
            // stops the test-owned racer if the isolated child fails or times out.
            let _ = worker.join();
        }
    }
}

fn replace(root: &Path, target: &Path) {
    let pending = root.join("next-link");
    std::os::unix::fs::symlink(target, &pending).unwrap();
    fs::rename(pending, root.join("switch")).unwrap();
}

fn wait_for(path: &Path, stop: &AtomicBool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !path.exists() {
        if stop.load(Ordering::Acquire) || Instant::now() >= deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(1));
    }
    true
}

pub(super) async fn run() {
    let fixture = TestWorkspace::create("landlock-symlink-race").unwrap();
    let outside = TestWorkspace::create("landlock-symlink-race-outside").unwrap();
    let inside = fixture.path().join("inside");
    fs::create_dir(&inside).unwrap();
    for (path, text) in [(inside.as_path(), "inside"), (outside.path(), "outside")] {
        fs::write(path.join("readable"), text).unwrap();
        fs::write(path.join("writable"), text).unwrap();
    }
    replace(fixture.path(), &inside);
    observation::print_environment(&inside, outside.path());
    let root = fixture.root().unwrap();
    let mut request = super::request(root.clone(), "symlink-race");
    request.authority.sandbox.filesystem.write_roots.push(root);
    request
        .env
        .push(("ROOT".into(), fixture.path().display().to_string()));
    let racer = Racer::start(fixture.path().to_owned(), inside, outside.path().to_owned());
    let output = super::execute_child(request).await;
    let swaps = racer.finish();
    println!("{}", String::from_utf8_lossy(&output.stdout));
    eprintln!("{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(
        fs::read_to_string(outside.path().join("writable")).unwrap(),
        "outside"
    );
    println!("ETAS_ISOLATION_SYMLINK swaps={swaps} outside_unchanged=true cleanup=settled");
    assert_eq!(output.exit_code, 0, "isolated race probe failed");
    assert!(swaps > 2, "the link must change during child execution");
}

pub(super) fn probe() {
    let root = PathBuf::from(std::env::var("ROOT").unwrap());
    let link = root.join("switch");
    let (inside_read, text) = read_through(&link).unwrap();
    assert_eq!(text, "inside");
    let inside_write = write_through(&link).unwrap();
    fs::write(root.join("ready"), "ready").unwrap();
    let stop = AtomicBool::new(false);
    assert!(wait_for(&root.join("outside-ready"), &stop));
    assert_eq!(
        read_through(&link).unwrap_err().error.raw_os_error(),
        Some(libc::EACCES)
    );
    assert_eq!(
        write_through(&link).unwrap_err().error.raw_os_error(),
        Some(libc::EACCES)
    );
    fs::write(root.join("outside-checked"), "ready").unwrap();
    assert!(wait_for(&root.join("racing"), &stop));
    let mut observations = Observations::default();
    for iteration in 0..4096 {
        match read_through(&link) {
            Ok((identity, text)) => {
                assert_eq!(
                    identity, inside_read,
                    "outside read descriptor escaped confinement"
                );
                assert_eq!(text, "inside", "outside data escaped confinement");
                observations.reads += 1;
            }
            Err(error) => observations.denied(iteration, &link, error),
        }
        match write_through(&link) {
            Ok(identity) => {
                assert_eq!(
                    identity, inside_write,
                    "outside write descriptor escaped confinement"
                );
                observations.writes += 1;
            }
            Err(error) => observations.denied(iteration, &link, error),
        }
    }
    observations.print();
}

pub(super) fn unconfined_control() {
    let fixture = TestWorkspace::create("symlink-rename-control").unwrap();
    let inside = fixture.path().join("inside");
    let outside = fixture.path().join("outside");
    fs::create_dir(&inside).unwrap();
    fs::create_dir(&outside).unwrap();
    for (path, text) in [(&inside, "inside"), (&outside, "outside")] {
        fs::write(path.join("readable"), text).unwrap();
    }
    replace(fixture.path(), &inside);
    let link = fixture.path().join("switch");
    let inside_id = read_through(&link).unwrap().0;
    replace(fixture.path(), &outside);
    let outside_id = read_through(&link).unwrap().0;
    let racer = Racer::start(fixture.path().to_owned(), inside, outside);
    fs::write(fixture.path().join("ready"), "ready").unwrap();
    fs::write(fixture.path().join("outside-checked"), "ready").unwrap();
    let ready = wait_for(&fixture.path().join("racing"), &AtomicBool::new(false));
    let mut inside_reads = 0;
    let mut outside_reads = 0;
    let result = (|| {
        if !ready {
            return Err(observation::Failure {
                stage: "control.wait_for_racing",
                error: std::io::Error::new(std::io::ErrorKind::TimedOut, "racer did not start"),
            });
        }
        for _ in 0..100_000 {
            let (identity, text) = read_through(&link)?;
            if identity == inside_id && text == "inside" {
                inside_reads += 1;
            } else if identity == outside_id && text == "outside" {
                outside_reads += 1;
            } else {
                return Err(observation::Failure {
                    stage: "control.identity",
                    error: std::io::Error::other("descriptor or content differs from both targets"),
                });
            }
        }
        Ok(())
    })();
    let swaps = racer.finish();
    println!(
        "ETAS_ISOLATION_SYMLINK_CONTROL {}",
        serde_json::json!({
            "inside_reads": inside_reads, "outside_reads": outside_reads, "swaps": swaps,
            "failure": result.as_ref().err().map(|failure| serde_json::json!({
                "stage": failure.stage, "errno": failure.error.raw_os_error(),
            })),
            "cleanup": "settled",
        })
    );
    result.unwrap();
    assert!(swaps > 2 && inside_reads > 0 && outside_reads > 0);
}
