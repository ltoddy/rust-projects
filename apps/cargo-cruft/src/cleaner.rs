use std::fs;
use std::path::{Path, PathBuf};

use crate::progress::Progress;
use crate::report::Candidate;

#[derive(Debug, Default)]
pub struct Cleanup {
    pub deleted: usize,
    pub errors: Vec<String>,
}

impl Cleanup {
    fn remove(&mut self, path: &Path) {
        let result = if path.is_dir() { fs::remove_dir_all(path) } else { fs::remove_file(path) };
        match result {
            Ok(()) => self.deleted += 1,
            Err(error) => self.errors.push(format!("{}: {error}", path.display())),
        }
    }
}

#[derive(Debug)]
pub struct Cleaner<'a> {
    candidates: &'a [Candidate],
    progress: &'a Progress,
}

impl<'a> Cleaner<'a> {
    pub fn new(candidates: &'a [Candidate], progress: &'a Progress) -> Self {
        Self { candidates, progress }
    }

    pub fn run(&self) -> Cleanup {
        let paths: Vec<&Path> = self
            .candidates
            .iter()
            .flat_map(|candidate| candidate.paths.iter().map(PathBuf::as_path))
            .collect();
        let mut cleanup = Cleanup::default();
        self.progress.begin("deleting", paths.len() as u64);
        for path in &paths {
            cleanup.remove(path);
            self.progress.advance();
        }
        for path in &paths {
            prune_empty_container(path);
        }
        self.progress.clear();
        cleanup
    }
}

fn prune_empty_container(path: &Path) {
    let Some(parent) = path.parent() else {
        return;
    };
    let container = parent.parent().and_then(Path::file_name).and_then(|name| name.to_str());
    if let Some("build" | "incremental") = container {
        let _ = fs::remove_dir(parent);
    }
}
