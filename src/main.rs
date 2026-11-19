mod args;
mod content;
mod count;
mod error;
mod progress;
mod report;
mod scan;
mod walk;

use clap::Parser;
use tokenizers::Tokenizer;

use crate::args::Args;
use crate::count::Counter;
use crate::error::Error;
use crate::progress::Progress;
use crate::report::Report;
use crate::scan::Scanner;
use crate::walk::Walker;

const TOKENIZER: &[u8] = include_bytes!("tokenizer.json");

fn main() -> Result<(), Error> {
    let args = Args::parse();

    let tokenizer = Tokenizer::from_bytes(TOKENIZER).map_err(Error::tokenizer)?;
    let root = args.path();
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());

    let scanner = Scanner::new(Counter::new(&tokenizer), threads);
    let progress = Progress::new()?;

    let walker = Walker::new(&root);
    let scan = scanner.scan(walker, &progress)?;

    progress.finish(scan.files());

    Report::new(root, scan).render(args.top);

    Ok(())
}
