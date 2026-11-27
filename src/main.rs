mod calculator;
mod engine;
mod executor;
mod explorer;
mod options;
mod pretty_printer;
mod reporter;
mod spinner;
mod util;

use std::env::current_dir;
use std::fs;
use std::time::Instant;

use clap::Parser;

use crate::engine::Engine;
use crate::options::{Options, Output, SortBy};
use crate::pretty_printer::{pretty_print, print_markdown};
use crate::util::compare;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    let now = Instant::now();
    let options = Options::parse();

    let entry = options.entry.and_then(|entry| fs::canonicalize(entry).ok()).unwrap_or_else(|| {
        eprintln!("No directory specified, so use current directory as entry.\n");
        current_dir().expect("current directory does not exist")
    });

    let engine = Engine::new(entry, options.ignore_file);
    let mut report = engine.serve();

    report.sections.sort_by(|prev, next| {
        let ordering = match options.sort_by {
            SortBy::Language => compare(prev.language, next.language, options.order_by),
            SortBy::Files => compare(prev.files, next.files, options.order_by),
            SortBy::Size => compare(prev.bytes, next.bytes, options.order_by),
            SortBy::Blank => compare(prev.blank, next.blank, options.order_by),
            SortBy::Comment => compare(prev.comment, next.comment, options.order_by),
            SortBy::Code => compare(prev.code, next.code, options.order_by),
        };

        // Keep the report stable when the sorted fields are equal.
        ordering.then_with(|| prev.language.cmp(next.language))
    });

    let elapsed = now.elapsed();

    match options.output {
        Output::Terminal => pretty_print(&report, elapsed),
        Output::Markdown => print_markdown(&report, elapsed),
    }
}
