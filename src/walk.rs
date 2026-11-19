use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use crossbeam_channel::{Receiver, Sender, bounded};
use ignore::{WalkBuilder, WalkParallel, WalkState};

pub struct Walker {
    inner: WalkParallel,
    tx: Sender<PathBuf>,
    rx: Receiver<PathBuf>,
}

impl Walker {
    const THREADS: usize = 2;

    pub fn new(path: impl AsRef<Path>) -> Self {
        let inner = WalkBuilder::new(path)
            .threads(Self::THREADS)
            .build_parallel();
        let (tx, rx) = bounded(256);

        Self { inner, tx, rx }
    }

    pub fn files(&self) -> Receiver<PathBuf> {
        self.rx.clone()
    }

    pub fn run(self) -> usize {
        let Self { inner, tx, rx: _ } = self;
        let failed = AtomicUsize::new(0);
        let failed_ref = &failed;

        inner.run(|| {
            Box::new(|entry| {
                let Ok(entry) = entry else {
                    failed_ref.fetch_add(1, Ordering::Relaxed);
                    return WalkState::Continue;
                };

                if !entry
                    .file_type()
                    .is_some_and(|file_type| file_type.is_file())
                {
                    return WalkState::Continue;
                }

                match tx.send(entry.path().to_path_buf()) {
                    Ok(_) => WalkState::Continue,
                    Err(_) => WalkState::Quit,
                }
            })
        });

        failed.into_inner()
    }
}
