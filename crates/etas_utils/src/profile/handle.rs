use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, Mutex},
};

use super::{
    io::write_profile_report,
    model::{ProfileReport, ProfileSpanStatus},
    recorder::ProfileRecorder,
};

#[derive(Clone, Debug, Default)]
pub struct ProfileHandle {
    inner: Option<Arc<Mutex<ProfileRecorder>>>,
}

impl ProfileHandle {
    pub fn disabled() -> Self {
        Self { inner: None }
    }

    pub fn enabled(command: impl Into<String>) -> Self {
        Self {
            inner: Some(Arc::new(Mutex::new(ProfileRecorder::new(command)))),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.inner.is_some()
    }

    pub fn span(&self, name: impl Into<String>, category: impl Into<String>) -> ProfileSpanGuard {
        let Some(inner) = &self.inner else {
            return ProfileSpanGuard::disabled();
        };
        let Ok(mut recorder) = inner.lock() else {
            return ProfileSpanGuard::disabled();
        };
        let id = recorder.start_span(name.into(), category.into(), None, BTreeMap::new());
        ProfileSpanGuard::enabled(self.clone(), id)
    }

    pub fn span_with_attrs(
        &self,
        name: impl Into<String>,
        category: impl Into<String>,
        attrs: BTreeMap<String, String>,
    ) -> ProfileSpanGuard {
        let Some(inner) = &self.inner else {
            return ProfileSpanGuard::disabled();
        };
        let Ok(mut recorder) = inner.lock() else {
            return ProfileSpanGuard::disabled();
        };
        let id = recorder.start_span(name.into(), category.into(), None, attrs);
        ProfileSpanGuard::enabled(self.clone(), id)
    }

    pub fn counter(&self, name: impl Into<String>, value: u64) {
        let Some(inner) = &self.inner else {
            return;
        };
        if let Ok(mut recorder) = inner.lock() {
            recorder.counter(name.into(), value);
        }
    }

    pub fn completed_span(
        &self,
        name: impl Into<String>,
        category: impl Into<String>,
        start_ns: u64,
        duration_ns: u64,
    ) {
        let Some(inner) = &self.inner else {
            return;
        };
        if let Ok(mut recorder) = inner.lock() {
            recorder.record_completed_span(
                name.into(),
                category.into(),
                None,
                BTreeMap::new(),
                start_ns,
                duration_ns,
                ProfileSpanStatus::Ok,
            );
        }
    }

    pub fn elapsed_ns(&self) -> Option<u64> {
        let inner = self.inner.as_ref()?;
        let recorder = inner.lock().ok()?;
        Some(recorder.elapsed_ns())
    }

    pub fn finish_report(&self, status: impl Into<String>) -> Option<ProfileReport> {
        let inner = self.inner.as_ref()?;
        let mut recorder = inner.lock().ok()?;
        Some(recorder.finish_report(status.into()))
    }

    pub fn write_report(
        &self,
        path: &Path,
        status: impl Into<String>,
    ) -> Result<(), std::io::Error> {
        let Some(report) = self.finish_report(status) else {
            return Ok(());
        };
        write_profile_report(path, &report)
    }

    fn finish_span(&self, id: u64, status: ProfileSpanStatus) {
        let Some(inner) = &self.inner else {
            return;
        };
        if let Ok(mut recorder) = inner.lock() {
            recorder.finish_span(id, status);
        }
    }
}

pub struct ProfileSpanGuard {
    handle: ProfileHandle,
    id: Option<u64>,
    finished: bool,
}

impl ProfileSpanGuard {
    fn disabled() -> Self {
        Self {
            handle: ProfileHandle::disabled(),
            id: None,
            finished: true,
        }
    }

    fn enabled(handle: ProfileHandle, id: u64) -> Self {
        Self {
            handle,
            id: Some(id),
            finished: false,
        }
    }

    pub fn finish_ok(mut self) {
        self.finish(ProfileSpanStatus::Ok);
    }

    pub fn finish_error(mut self) {
        self.finish(ProfileSpanStatus::Error);
    }

    pub fn finish(&mut self, status: ProfileSpanStatus) {
        if self.finished {
            return;
        }
        if let Some(id) = self.id {
            self.handle.finish_span(id, status);
        }
        self.finished = true;
    }
}

impl Drop for ProfileSpanGuard {
    fn drop(&mut self) {
        self.finish(ProfileSpanStatus::Ok);
    }
}
