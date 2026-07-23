use std::{
    collections::BTreeMap,
    time::{Instant, SystemTime},
};

use super::model::{ProfileCounter, ProfileReport, ProfileSpan, ProfileSpanStatus};

#[derive(Clone, Copy, Debug)]
pub(crate) struct CompletedSpanTiming {
    pub(crate) start_ns: u64,
    pub(crate) duration_ns: u64,
}

#[derive(Clone, Debug)]
pub struct ProfileRecorder {
    command: String,
    started_at: SystemTime,
    started: Instant,
    next_span_id: u64,
    spans: Vec<ProfileSpan>,
    counters: Vec<ProfileCounter>,
}

impl ProfileRecorder {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            started_at: SystemTime::now(),
            started: Instant::now(),
            next_span_id: 1,
            spans: Vec::new(),
            counters: Vec::new(),
        }
    }

    pub fn start_span(
        &mut self,
        name: String,
        category: String,
        parent: Option<u64>,
        attrs: BTreeMap<String, String>,
    ) -> u64 {
        let id = self.next_span_id;
        self.next_span_id = self.next_span_id.saturating_add(1);
        self.spans.push(ProfileSpan {
            id,
            parent,
            name,
            category,
            start_ns: self.started.elapsed().as_nanos() as u64,
            duration_ns: None,
            status: ProfileSpanStatus::Running,
            attrs,
        });
        id
    }

    pub(crate) fn record_completed_span(
        &mut self,
        name: String,
        category: String,
        parent: Option<u64>,
        attrs: BTreeMap<String, String>,
        timing: CompletedSpanTiming,
        status: ProfileSpanStatus,
    ) -> u64 {
        let id = self.next_span_id;
        self.next_span_id = self.next_span_id.saturating_add(1);
        self.spans.push(ProfileSpan {
            id,
            parent,
            name,
            category,
            start_ns: timing.start_ns,
            duration_ns: Some(timing.duration_ns),
            status,
            attrs,
        });
        id
    }

    pub fn finish_span(&mut self, id: u64, status: ProfileSpanStatus) {
        if let Some(span) = self.spans.iter_mut().find(|span| span.id == id)
            && span.duration_ns.is_none()
        {
            span.duration_ns = Some(
                self.started
                    .elapsed()
                    .as_nanos()
                    .saturating_sub(span.start_ns as u128) as u64,
            );
            span.status = status;
        }
    }

    pub fn counter(&mut self, name: String, value: u64) {
        self.counters.push(ProfileCounter { name, value });
    }

    pub fn elapsed_ns(&self) -> u64 {
        self.started.elapsed().as_nanos() as u64
    }

    pub fn finish_report(&mut self, status: String) -> ProfileReport {
        let elapsed = self.started.elapsed().as_nanos() as u64;
        for span in &mut self.spans {
            if span.duration_ns.is_none() {
                span.duration_ns =
                    Some((elapsed as u128).saturating_sub(span.start_ns as u128) as u64);
                span.status = ProfileSpanStatus::Abandoned;
            }
        }
        ProfileReport {
            schema: "etas.profile.v1".to_owned(),
            command: self.command.clone(),
            status,
            started_at_unix_ms: self
                .started_at
                .duration_since(SystemTime::UNIX_EPOCH)
                .ok()
                .map(|duration| duration.as_millis() as u64),
            total_duration_ns: elapsed,
            spans: self.spans.clone(),
            counters: self.counters.clone(),
        }
    }
}
