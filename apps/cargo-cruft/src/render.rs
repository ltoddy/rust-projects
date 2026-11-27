use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::bytes::Bytes;
use crate::cleaner::Cleanup;
use crate::report::{Kind, Report};

const ROWS_PER_KIND: usize = 12;
const ERRORS_SHOWN: usize = 10;

#[derive(Debug, Default)]
struct Row {
    count: usize,
    bytes: u64,
}

impl Row {
    fn add(&mut self, bytes: u64) {
        self.count += 1;
        self.bytes += bytes;
    }
}

pub fn report(report: &Report) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "target: {}", report.target_root.display());
    let _ = writeln!(out, "size:   {}", Bytes(report.total_bytes));
    if report.candidates.is_empty() {
        let _ = writeln!(out, "\nnothing to clean");
        return out;
    }
    for kind in Kind::ALL {
        render_kind(&mut out, report, kind);
    }
    let _ = writeln!(
        out,
        "\nreclaimable: {} of {}",
        Bytes(report.reclaimable()),
        Bytes(report.total_bytes)
    );
    out
}

fn render_kind(out: &mut String, report: &Report, kind: Kind) {
    let mut by_profile: BTreeMap<&str, BTreeMap<&str, Row>> = BTreeMap::new();
    for candidate in report.candidates.iter().filter(|candidate| candidate.kind == kind) {
        by_profile
            .entry(&candidate.profile)
            .or_default()
            .entry(&candidate.label)
            .or_default()
            .add(candidate.reclaimable);
    }
    if by_profile.is_empty() {
        return;
    }
    let _ = writeln!(out, "\n{}:", title(kind));
    for (profile, rows) in by_profile {
        let _ = writeln!(out, "  {profile}:");
        let mut rows: Vec<(&str, Row)> = rows.into_iter().collect();
        rows.sort_by_key(|(_, row)| std::cmp::Reverse(row.bytes));
        for (label, row) in rows.iter().take(ROWS_PER_KIND) {
            let _ = writeln!(
                out,
                "    {label:<44} {:>4} {}{} {:>10}",
                row.count,
                noun(kind),
                if row.count == 1 { "" } else { "s" },
                Bytes(row.bytes)
            );
        }
        if rows.len() > ROWS_PER_KIND {
            let _ = writeln!(out, "    ... {} more", rows.len() - ROWS_PER_KIND);
        }
    }
}

fn title(kind: Kind) -> &'static str {
    match kind {
        Kind::RemovedPackage => "packages that left Cargo.lock",
        Kind::MissingArtifacts => "units whose artifacts are gone",
        Kind::Unowned => "leftovers no build owns",
        Kind::Incremental => "incremental data",
    }
}

fn noun(kind: Kind) -> &'static str {
    match kind {
        Kind::Incremental => "config",
        _ => "unit",
    }
}

#[derive(Debug)]
pub struct CleanupText {
    pub summary: String,
    pub details: String,
}

pub fn cleanup(cleanup: &Cleanup, reclaimable: Bytes) -> CleanupText {
    if cleanup.errors.is_empty() {
        return CleanupText {
            summary: format!("deleted {} paths, freed {reclaimable}\n", cleanup.deleted),
            details: String::new(),
        };
    }
    let summary = format!(
        "deleted {} paths, freed up to {reclaimable}; {} errors\n",
        cleanup.deleted,
        cleanup.errors.len()
    );
    let mut details = String::new();
    for error in cleanup.errors.iter().take(ERRORS_SHOWN) {
        let _ = writeln!(details, "  {error}");
    }
    if cleanup.errors.len() > ERRORS_SHOWN {
        let _ = writeln!(details, "  ... {} more", cleanup.errors.len() - ERRORS_SHOWN);
    }
    CleanupText { summary, details }
}
