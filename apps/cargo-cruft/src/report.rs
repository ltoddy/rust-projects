use std::path::PathBuf;

use crate::files::{FileStat, collect_files};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    RemovedPackage,
    MissingArtifacts,
    Unowned,
    Incremental,
}

impl Kind {
    pub const ALL: [Self; 4] =
        [Self::RemovedPackage, Self::MissingArtifacts, Self::Unowned, Self::Incremental];
}

#[derive(Debug)]
pub struct Artifact {
    pub label: String,
    pub paths: Vec<PathBuf>,
    pub files: Vec<FileStat>,
}

impl Artifact {
    pub fn from_directory(dir: PathBuf, label: String) -> Self {
        let files = collect_files(&dir).into_iter().map(|entry| entry.stat).collect();
        Self { label, paths: vec![dir], files }
    }
}

#[derive(Debug)]
pub struct Candidate {
    pub kind: Kind,
    pub profile: String,
    pub label: String,
    pub paths: Vec<PathBuf>,
    pub files: Vec<FileStat>,
    pub reclaimable: u64,
}

impl Candidate {
    pub fn classify(kind: Kind, profile: &str, artifact: Artifact) -> Self {
        Self {
            kind,
            profile: profile.to_owned(),
            label: artifact.label,
            paths: artifact.paths,
            files: artifact.files,
            reclaimable: 0,
        }
    }
}

#[derive(Debug)]
pub struct Report {
    pub target_root: PathBuf,
    pub candidates: Vec<Candidate>,
    pub total_bytes: u64,
}

impl Report {
    pub fn new(target_root: PathBuf, candidates: Vec<Candidate>, total_bytes: u64) -> Self {
        Self { target_root, candidates, total_bytes }
    }

    pub fn reclaimable(&self) -> u64 {
        self.candidates.iter().map(|candidate| candidate.reclaimable).sum()
    }
}
