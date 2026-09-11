use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use etas_host::TestWorkspace;

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
    let root = fixture.root().unwrap();
    let mut request = super::request(root.clone(), "symlink-race");
    request.authority.sandbox.filesystem.write_roots.push(root);
    request
        .env
        .push(("ROOT".into(), fixture.path().display().to_string()));
    let racer = Racer::start(fixture.path().to_owned(), inside, outside.path().to_owned());
    super::execute(request).await;
    let swaps = racer.finish();
    assert!(swaps > 2, "the link must change during child execution");
    assert_eq!(
        fs::read_to_string(outside.path().join("writable")).unwrap(),
        "outside"
    );
    println!("ETAS_ISOLATION_SYMLINK swaps={swaps} outside_unchanged=true cleanup=settled");
}

fn write_through(link: &Path) -> std::io::Result<()> {
    fs::OpenOptions::new()
        .write(true)
        .open(link.join("writable"))?
        .write_all(b"inside")
}

pub(super) fn probe() {
    let root = PathBuf::from(std::env::var("ROOT").unwrap());
    let link = root.join("switch");
    assert_eq!(fs::read_to_string(link.join("readable")).unwrap(), "inside");
    write_through(&link).unwrap();
    fs::write(root.join("ready"), "ready").unwrap();
    let stop = AtomicBool::new(false);
    assert!(wait_for(&root.join("outside-ready"), &stop));
    assert_eq!(
        fs::read(link.join("readable")).unwrap_err().raw_os_error(),
        Some(libc::EACCES)
    );
    assert_eq!(
        write_through(&link).unwrap_err().raw_os_error(),
        Some(libc::EACCES)
    );
    fs::write(root.join("outside-checked"), "ready").unwrap();
    assert!(wait_for(&root.join("racing"), &stop));
    for _ in 0..4096 {
        match fs::read_to_string(link.join("readable")) {
            Ok(text) => assert_eq!(text, "inside", "outside data escaped confinement"),
            Err(error) => assert_eq!(error.raw_os_error(), Some(libc::EACCES)),
        }
        if let Err(error) = write_through(&link) {
            assert_eq!(error.raw_os_error(), Some(libc::EACCES));
        }
    }
}
