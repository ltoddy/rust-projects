mod args;
mod content;
mod count;
mod error;
mod progress;
mod report;
mod scan;
mod walk;

use std::thread;

use clap::Parser;

use crate::args::Args;
use crate::error::Error;
use crate::progress::Progress;
use crate::report::Report;
use crate::scan::Scanner;
use crate::walk::Walker;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> Result<(), Error> {
    let args = Args::parse();
    let root = args.path();

    if !root.exists() {
        return Err(Error::missing(&root));
    }

    let threads = thread::available_parallelism().map_or(1, |count| count.get());
    let progress = Progress::new()?;
    let scanner = Scanner::new(threads);
    let walker = Walker::new(&root);

    let scan = scanner.scan(walker, &progress)?;

    progress.finish(scan.files());

    Report::new(root, scan).render(&args);

    Ok(())
}
