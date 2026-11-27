use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use walkdir::WalkDir;

use crate::files::{FileKey, FileStat};
use crate::incremental;
use crate::ledger::Ledger;
use crate::lockfile::LockSet;
use crate::progress::Progress;
use crate::report::{Artifact, Candidate, Kind, Report};
use crate::target::{Profile, TargetDir};
use crate::unit::{Unit, build_layout};

#[derive(Debug)]
pub struct Scanner<'a> {
    target: &'a TargetDir,
    lock: Option<&'a LockSet>,
    progress: &'a Progress,
}

impl<'a> Scanner<'a> {
    pub fn new(target: &'a TargetDir, lock: Option<&'a LockSet>, progress: &'a Progress) -> Self {
        Self { target, lock, progress }
    }

    pub fn scan(&self) -> Report {
        let mut candidates = Vec::new();
        let mut swept: HashSet<PathBuf> = HashSet::new();
        let mut built_profiles = Vec::new();

        for profile in self.target.profiles() {
            if self.scan_profile(profile, &mut candidates, &mut swept) {
                built_profiles.push(profile);
            }
        }
        for profile in built_profiles {
            for artifact in incremental::scan(profile) {
                candidates.push(Candidate::classify(Kind::Incremental, profile.label(), artifact));
            }
        }

        let ledger = self.resolve_links(&mut candidates);
        let total_bytes = self.measure_target(&ledger, &swept);
        self.progress.clear();
        Report::new(self.target.root().to_path_buf(), candidates, total_bytes)
    }

    fn scan_profile(
        &self,
        profile: &Profile,
        candidates: &mut Vec<Candidate>,
        swept: &mut HashSet<PathBuf>,
    ) -> bool {
        let layout = build_layout(profile);
        self.progress.begin(&format!("scanning {}", profile.label()), layout.units.len() as u64);
        let mut units = Vec::new();
        for dir in &layout.units {
            if let Some(unit) = Unit::probe(dir) {
                units.push(unit);
            }
            self.progress.advance();
        }

        let has_units = !units.is_empty();
        for unit in units {
            let Some(kind) = self.classify(&unit) else {
                continue;
            };
            swept.insert(unit.dir().to_path_buf());
            candidates.push(Candidate::classify(kind, profile.label(), unit.into_artifact()));
        }
        for dir in layout.leftovers {
            let label =
                dir.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
            swept.insert(dir.clone());
            candidates.push(Candidate::classify(
                Kind::Unowned,
                profile.label(),
                Artifact::from_directory(dir, label),
            ));
        }
        has_units
    }

    fn classify(&self, unit: &Unit) -> Option<Kind> {
        if let Some(lock) = self.lock
            && !lock.contains_package(unit.name(), unit.version())
        {
            return Some(Kind::RemovedPackage);
        }
        unit.artifacts_missing().then_some(Kind::MissingArtifacts)
    }

    fn resolve_links(&self, candidates: &mut Vec<Candidate>) -> Ledger {
        let mut ledger = Ledger::new(candidates.as_slice());
        let mut linked: Vec<(usize, PathBuf)> = Vec::new();
        for path in self.target.profiles().iter().flat_map(Profile::linked_files) {
            let Ok(metadata) = fs::symlink_metadata(&path) else {
                continue;
            };
            if !metadata.is_file() {
                continue;
            }
            let stat = FileStat::of(&path, &metadata);
            if let Some(owner) = ledger.claim(&stat) {
                linked.push((owner, path));
            }
        }

        let reclaimable = ledger.reclaimable(candidates.len());
        for (candidate, bytes) in candidates.iter_mut().zip(reclaimable) {
            candidate.reclaimable = bytes;
        }
        for (owner, path) in linked {
            candidates[owner].paths.push(path);
        }
        ledger
    }

    fn measure_target(&self, ledger: &Ledger, swept: &HashSet<PathBuf>) -> u64 {
        let mut seen: HashSet<FileKey> = ledger.keys().collect();
        let mut total = ledger.total_bytes();
        self.progress.begin("measuring target", 0);
        let files = WalkDir::new(self.target.root())
            .into_iter()
            .filter_entry(|entry| !swept.contains(entry.path()))
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file());
        for entry in files {
            let Ok(metadata) = fs::symlink_metadata(entry.path()) else {
                continue;
            };
            let stat = FileStat::of(entry.path(), &metadata);
            if seen.insert(stat.key()) {
                total += stat.bytes();
            }
            self.progress.advance();
        }
        total
    }
}
