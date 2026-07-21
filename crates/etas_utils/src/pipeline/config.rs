use std::collections::BTreeSet;

#[derive(Clone, Debug, Default)]
pub struct PipelineConfig {
    enabled: BTreeSet<&'static str>,
    disabled: BTreeSet<&'static str>,
    stop_before: Option<&'static str>,
    stop_after: Option<&'static str>,
    pub fail_fast: bool,
    pub collect_timing: bool,
    pub collect_stats: bool,
}

impl PipelineConfig {
    pub fn enable_only(mut self, pass: &'static str) -> Self {
        self.enabled.insert(pass);
        self
    }

    pub fn disable(mut self, pass: &'static str) -> Self {
        self.disabled.insert(pass);
        self
    }

    pub fn stop_before(mut self, pass: &'static str) -> Self {
        self.stop_before = Some(pass);
        self
    }

    pub fn stop_after(mut self, pass: &'static str) -> Self {
        self.stop_after = Some(pass);
        self
    }

    pub fn with_timing(mut self, enabled: bool) -> Self {
        self.collect_timing = enabled;
        self
    }

    pub fn with_stats(mut self, enabled: bool) -> Self {
        self.collect_stats = enabled;
        self
    }

    pub fn should_run(&self, pass: &'static str) -> bool {
        !self.disabled.contains(pass) && (self.enabled.is_empty() || self.enabled.contains(pass))
    }

    pub fn should_stop_before(&self, pass: &'static str) -> bool {
        self.stop_before == Some(pass)
    }

    pub fn should_stop_after(&self, pass: &'static str) -> bool {
        self.stop_after == Some(pass)
    }
}
