mod args;
mod error;

use clap::Parser;
use crossbeam_channel::{Sender, bounded};
use ignore::{WalkBuilder, WalkState};
use std::path::{Path, PathBuf};
use tokenizers::Tokenizer;

use crate::args::Args;
use crate::error::Error;

const TOKENIZER: &[u8] = include_bytes!("tokenizer.json");

fn main() -> Result<(), Error> {
    let args = Args::parse();

    let tokenizer = Tokenizer::from_bytes(TOKENIZER).map_err(Error::tokenizer)?;

    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    let (sender, receiver) = bounded::<PathBuf>(256);

    let walk = std::thread::spawn(move || {
        walk(args.path(), sender, threads);
    });

    std::thread::scope(|scope| {
        let _ = (0..threads)
            .map(|_| {
                let receiver = receiver.clone();
                let tokenizer = &tokenizer;

                scope.spawn(move || -> Result<(), Error> {
                    for file_path in receiver.iter() {
                        println!("file_path: {}", file_path.display());

                        let text = std::fs::read_to_string(&file_path)
                            .map_err(|source| Error::read_file(file_path.clone(), source))?;

                        let encoding = tokenizer
                            .encode(text.as_str(), false)
                            .map_err(|source| Error::encode(file_path.clone(), source))?;

                        println!("{}: {} tokens", file_path.display(), encoding.len());
                    }

                    Ok(())
                })
            })
            .collect::<Vec<_>>();
    });
    let _ = walk.join();

    Ok(())
}

pub fn walk(root: impl AsRef<Path>, sender: Sender<PathBuf>, threads: usize) {
    let walker = WalkBuilder::new(root).threads(threads).build_parallel();

    walker.run(|| {
        Box::new(|entry| {
            let Ok(entry) = entry else {
                return WalkState::Continue;
            };

            if !entry
                .file_type()
                .is_some_and(|file_type| file_type.is_file())
            {
                return WalkState::Continue;
            }

            match sender.send(entry.path().to_path_buf()) {
                Ok(()) => WalkState::Continue,
                Err(_) => WalkState::Quit,
            }
        })
    });
}
