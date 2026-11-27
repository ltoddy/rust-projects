use std::fs::{self, File};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::files::{entry_is_dir, entry_name};

const IGNORED_DIRS: [&str; 5] = ["doc", "package", "tmp", "flycheck0", "criterion"];
const PROFILE_MARKERS: [&str; 5] = [".cargo-lock", "build", "incremental", "deps", ".fingerprint"];

#[derive(Debug)]
pub struct TargetDir {
    root: PathBuf,
    profiles: Vec<Profile>,
}

impl TargetDir {
    pub fn open(root: &Path) -> Option<Self> {
        if !root.is_dir() {
            return None;
        }
        Some(Self { root: root.to_path_buf(), profiles: discover_profiles(root) })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn profiles(&self) -> &[Profile] {
        &self.profiles
    }
}

#[derive(Debug, Clone)]
pub struct Profile {
    dir: PathBuf,
    label: String,
}

impl Profile {
    fn new(root: &Path, dir: PathBuf) -> Self {
        let label = dir.strip_prefix(root).unwrap_or(&dir).to_string_lossy().replace('\\', "/");
        Self { dir, label }
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn build_dir(&self) -> PathBuf {
        self.dir.join("build")
    }

    pub fn incremental_dir(&self) -> PathBuf {
        self.dir.join("incremental")
    }

    pub fn linked_files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();
        for dir in [self.dir.clone(), self.dir.join("examples")] {
            let Ok(entries) = fs::read_dir(dir) else {
                continue;
            };
            for entry in entries.flatten() {
                if entry.file_type().is_ok_and(|kind| kind.is_file()) {
                    files.push(entry.path());
                }
            }
        }
        files
    }
}

#[derive(Debug)]
pub struct TargetGuard {
    _locks: Vec<File>,
}

impl TargetGuard {
    pub fn acquire(profiles: &[Profile]) -> Result<Self> {
        let mut locks = Vec::with_capacity(profiles.len());
        for profile in profiles {
            let path = profile.dir().join(".cargo-lock");
            let file =
                File::open(&path).with_context(|| format!("cannot open {}", path.display()))?;
            file.try_lock()
                .with_context(|| format!("{} is locked by a running cargo", path.display()))?;
            locks.push(file);
        }
        Ok(Self { _locks: locks })
    }
}

fn discover_profiles(root: &Path) -> Vec<Profile> {
    let mut profiles = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return profiles;
    };
    for entry in entries.flatten() {
        if !entry_is_dir(&entry) || IGNORED_DIRS.contains(&entry_name(&entry).as_str()) {
            continue;
        }
        let path = entry.path();
        if is_profile(&path) {
            profiles.push(Profile::new(root, path));
            continue;
        }
        let Ok(children) = fs::read_dir(&path) else {
            continue;
        };
        for child in children.flatten() {
            if entry_is_dir(&child) && is_profile(&child.path()) {
                profiles.push(Profile::new(root, child.path()));
            }
        }
    }
    profiles.sort_by(|left, right| left.dir.cmp(&right.dir));
    profiles
}

fn is_profile(path: &Path) -> bool {
    PROFILE_MARKERS.iter().any(|marker| path.join(marker).exists())
}
