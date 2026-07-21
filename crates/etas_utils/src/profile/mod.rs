mod handle;
mod io;
mod model;
mod recorder;
mod render;

pub use handle::{ProfileHandle, ProfileSpanGuard};
pub use io::write_profile_report;
pub use model::{ProfileCounter, ProfileReport, ProfileSpan, ProfileSpanStatus};
pub use recorder::ProfileRecorder;
pub use render::{ProfileTreeRenderOptions, render_profile_tree, render_profile_tree_with_options};

#[cfg(test)]
mod tests {
    use super::{ProfileHandle, ProfileSpanStatus};

    #[test]
    fn disabled_profile_is_noop() {
        let profile = ProfileHandle::disabled();
        let span = profile.span("ignored", "test");
        span.finish_ok();
        profile.counter("ignored", 1);

        assert!(profile.finish_report("ok").is_none());
    }

    #[test]
    fn enabled_profile_records_spans_and_counters() {
        let profile = ProfileHandle::enabled("check");
        let span = profile.span("frontend.check", "frontend");
        profile.counter("frontend.source_files", 2);
        span.finish_error();

        let report = profile.finish_report("error").expect("profile report");
        assert_eq!(report.schema, "etas.profile.v1");
        assert_eq!(report.command, "check");
        assert_eq!(report.status, "error");
        assert_eq!(report.spans.len(), 1);
        assert_eq!(report.spans[0].name, "frontend.check");
        assert_eq!(report.spans[0].status, ProfileSpanStatus::Error);
        assert_eq!(report.counters.len(), 1);
        assert_eq!(report.counters[0].name, "frontend.source_files");
        assert_eq!(report.counters[0].value, 2);
    }
}
