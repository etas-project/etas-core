use std::fmt::Write as _;

use super::model::{ProfileReport, ProfileSpan};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProfileTreeRenderOptions {
    pub include_detail: bool,
    pub include_pass_timing: bool,
}

pub fn render_profile_tree(report: &ProfileReport) -> String {
    render_profile_tree_with_options(report, ProfileTreeRenderOptions::default())
}

pub fn render_profile_tree_with_options(
    report: &ProfileReport,
    options: ProfileTreeRenderOptions,
) -> String {
    let mut output = String::new();
    let _ = writeln!(
        output,
        "profile {} status={} total={}",
        report.command,
        report.status,
        format_duration_ns(report.total_duration_ns)
    );

    let children = inferred_span_children(&report.spans);
    for (index, parent) in inferred_span_parents(&report.spans).iter().enumerate() {
        if parent.is_none() {
            render_span_tree(
                &mut output,
                report,
                &children,
                RenderSpanTreeState {
                    index,
                    prefix: "",
                    last: true,
                    depth: 0,
                    include_detail: options.include_detail,
                },
            );
        }
    }

    let pass_count = pass_timing_counters(report).len();
    if pass_count > 0 {
        let _ = writeln!(
            output,
            "pipeline passes: {pass_count} timed pass{}{}",
            if pass_count == 1 { "" } else { "es" },
            if options.include_pass_timing {
                ""
            } else {
                " (use --profile-pass-timing for details)"
            }
        );
    }

    if !options.include_pass_timing {
        return output;
    }

    render_pass_timing(&mut output, report);
    output
}

fn render_pass_timing(output: &mut String, report: &ProfileReport) {
    let pass_counters = pass_timing_counters(report);
    if !pass_counters.is_empty() {
        let _ = writeln!(output, "pipeline passes");
        for counter in pass_counters {
            let _ = writeln!(
                output,
                "  {} {}",
                pass_counter_label(&counter.name),
                format_duration_ns(counter.value)
            );
        }
    }
}

fn pass_timing_counters(report: &ProfileReport) -> Vec<&super::model::ProfileCounter> {
    report
        .counters
        .iter()
        .filter(|counter| {
            counter.name.starts_with("frontend.pass.") || counter.name.starts_with("effects.pass.")
        })
        .collect()
}

fn pass_counter_label(name: &str) -> &str {
    name.strip_prefix("frontend.pass.")
        .or_else(|| name.strip_prefix("effects.pass."))
        .and_then(|name| name.strip_suffix(".duration_ns"))
        .unwrap_or(name)
}

fn inferred_span_parents(spans: &[ProfileSpan]) -> Vec<Option<usize>> {
    let mut parents = Vec::with_capacity(spans.len());
    for (index, span) in spans.iter().enumerate() {
        if let Some(parent_id) = span.parent
            && let Some(parent_index) = spans.iter().position(|candidate| candidate.id == parent_id)
        {
            parents.push(Some(parent_index));
            continue;
        }
        let span_start = span.start_ns;
        let span_end = span_end_ns(span);
        let mut parent = None;
        for (candidate_index, candidate) in spans.iter().enumerate() {
            if candidate_index == index {
                continue;
            }
            let candidate_start = candidate.start_ns;
            let candidate_end = span_end_ns(candidate);
            if candidate_start <= span_start
                && span_end <= candidate_end
                && (candidate_start < span_start || span_end < candidate_end)
                && match parent {
                    Some(parent_index) => {
                        span_duration_ns(candidate) < span_duration_ns(&spans[parent_index])
                    }
                    None => true,
                }
            {
                parent = Some(candidate_index);
            }
        }
        parents.push(parent);
    }
    parents
}

fn inferred_span_children(spans: &[ProfileSpan]) -> Vec<Vec<usize>> {
    let parents = inferred_span_parents(spans);
    let mut children = vec![Vec::new(); spans.len()];
    for (index, parent) in parents.into_iter().enumerate() {
        if let Some(parent) = parent {
            children[parent].push(index);
        }
    }
    for children in &mut children {
        children.sort_by_key(|index| spans[*index].start_ns);
    }
    children
}

struct RenderSpanTreeState<'a> {
    index: usize,
    prefix: &'a str,
    last: bool,
    depth: usize,
    include_detail: bool,
}

fn render_span_tree(
    output: &mut String,
    report: &ProfileReport,
    children: &[Vec<usize>],
    state: RenderSpanTreeState<'_>,
) {
    let index = state.index;
    let span = &report.spans[index];
    let branch = if state.last { "`- " } else { "|- " };
    let _ = writeln!(
        output,
        "{}{branch}{} [{}] {} {:?}",
        state.prefix,
        span.name,
        span.category,
        format_duration_ns(span_duration_ns(span)),
        span.status
    );
    if !state.include_detail && state.depth >= 1 {
        let hidden = children[index].len();
        if hidden > 0 {
            let child_prefix = if state.last {
                format!("{}   ", state.prefix)
            } else {
                format!("{}|  ", state.prefix)
            };
            let _ = writeln!(
                output,
                "{child_prefix}`- ... {hidden} nested span{} hidden (use --profile-detail)",
                if hidden == 1 { "" } else { "s" }
            );
        }
        return;
    }
    let child_prefix = if state.last {
        format!("{}   ", state.prefix)
    } else {
        format!("{}|  ", state.prefix)
    };
    for (position, child) in children[index].iter().enumerate() {
        render_span_tree(
            output,
            report,
            children,
            RenderSpanTreeState {
                index: *child,
                prefix: &child_prefix,
                last: position + 1 == children[index].len(),
                depth: state.depth + 1,
                include_detail: state.include_detail,
            },
        );
    }
}

fn span_end_ns(span: &ProfileSpan) -> u64 {
    span.start_ns.saturating_add(span_duration_ns(span))
}

fn span_duration_ns(span: &ProfileSpan) -> u64 {
    span.duration_ns.unwrap_or_default()
}

fn format_duration_ns(ns: u64) -> String {
    if ns >= 1_000_000_000 {
        format!("{:.3}s", ns as f64 / 1_000_000_000.0)
    } else if ns >= 1_000_000 {
        format!("{:.3}ms", ns as f64 / 1_000_000.0)
    } else if ns >= 1_000 {
        format!("{:.3}us", ns as f64 / 1_000.0)
    } else {
        format!("{ns}ns")
    }
}
