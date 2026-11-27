use std::fs;
use std::path::PathBuf;
use std::thread::spawn;

use crate::calculator::Calculator;
use crate::explorer::Explorer;
use crate::reporter::{Report, Reporter};
use crate::spinner::Spinner;

pub struct Engine {
    entry: PathBuf,

    explorer: Explorer,
    calculator: Calculator,
    reporter: Reporter,
}

impl Engine {
    pub fn new(entry: PathBuf, ignore_file: Option<PathBuf>) -> Self {
        let ignore_list = Self::read_ignore_list(ignore_file);

        let (explorer, filename_receiver) = Explorer::new(ignore_list);
        let (calculator, detail_receiver) = Calculator::new(filename_receiver);
        let reporter = Reporter::new(detail_receiver);

        Self { entry, explorer, calculator, reporter }
    }

    pub fn serve(self) -> Report {
        let Self { entry, mut explorer, calculator, reporter } = self;
        let mut spinner = Spinner::new();

        spinner.start();
        spawn(move || explorer.walk_directory(&entry));
        spawn(move || calculator.calculate());
        let report = reporter.research();
        spinner.stop();

        report
    }

    fn read_ignore_list(filename: Option<PathBuf>) -> Option<Vec<PathBuf>> {
        filename.and_then(|filename| fs::read_to_string(filename).ok()).map(|content| {
            content
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .filter_map(Self::canonicalize_ignore_entry)
                .collect()
        })
    }

    /// Entries are usually written relative to the working directory, but absolute paths are accepted as well.
    fn canonicalize_ignore_entry(entry: &str) -> Option<PathBuf> {
        fs::canonicalize(entry).or_else(|_| fs::canonicalize(entry.trim_start_matches('/'))).ok()
    }
}
