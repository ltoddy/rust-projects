use std::path::PathBuf;

use crossbeam_channel::Receiver;

use crate::count::Counter;
use crate::error::Error;
use crate::progress::Progress;
use crate::report::{ScanResult, Stat, Tokens};
use crate::walk::Walker;

pub struct Scanner<'a> {
    counter: Counter<'a>,
    threads: usize,
}

impl<'a> Scanner<'a> {
    pub fn new(counter: Counter<'a>, threads: usize) -> Self {
        Self { counter, threads }
    }

    pub fn scan(&self, walker: Walker, progress: &Progress) -> Result<ScanResult, Error> {
        let files = walker.files();

        std::thread::scope(|scope| -> Result<ScanResult, Error> {
            let walker = scope.spawn(move || walker.run());

            let handles = (0..self.threads)
                .map(|_| {
                    let files = files.clone();
                    scope.spawn(move || self.consume(files, progress))
                })
                .collect::<Vec<_>>();

            let mut result = ScanResult::new(Vec::new(), 0, 0);

            for handle in handles {
                let (stats, skipped) = handle.join().map_err(|_| Error::thread())?;
                result.absorb(stats, skipped);
            }

            let dir_errors = walker.join().map_err(|_| Error::thread())?;

            Ok(result.with_dir_errors(dir_errors))
        })
    }

    fn consume(&self, files: Receiver<PathBuf>, progress: &Progress) -> (Vec<Stat>, usize) {
        let mut stats = Vec::new();
        let mut skipped = 0;

        for path in files.iter() {
            progress.report(&path);

            match self.counter.count(&path) {
                Ok(tokens) => stats.push(Stat::new(path, Tokens::new(tokens))),
                Err(_) => skipped += 1,
            }

            progress.advance();
        }

        (stats, skipped)
    }
}
