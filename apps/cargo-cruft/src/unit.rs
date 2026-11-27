use std::fs;
use std::path::{Component, Path, PathBuf};

use walkdir::WalkDir;

use crate::files::{FileEntry, FileStat, collect_files, entry_is_dir, entry_name};
use crate::report::Artifact;
use crate::target::Profile;

const UNIT_HASH_LEN: usize = 16;
const REGISTRY_PATH_LEN: usize = 4;

#[derive(Debug, Default)]
pub struct BuildLayout {
    pub units: Vec<PathBuf>,
    pub leftovers: Vec<PathBuf>,
}

pub fn build_layout(profile: &Profile) -> BuildLayout {
    let mut layout = BuildLayout::default();
    let Ok(packages) = fs::read_dir(profile.build_dir()) else {
        return layout;
    };
    for package in packages.flatten() {
        let Ok(hashes) = fs::read_dir(package.path()) else {
            continue;
        };
        for hash in hashes.flatten() {
            if !entry_is_dir(&hash) || !is_unit_hash(&entry_name(&hash)) {
                continue;
            }
            let dir = hash.path();
            if dir.join("fingerprint").is_dir() {
                layout.units.push(dir);
            } else {
                layout.leftovers.push(dir);
            }
        }
    }
    layout
}

#[derive(Debug)]
pub struct Unit {
    name: String,
    version: Option<String>,
    dir: PathBuf,
    files: Vec<FileStat>,
    runs_build_script: bool,
}

impl Unit {
    pub fn probe(dir: &Path) -> Option<Self> {
        let name = dir.parent()?.file_name()?.to_string_lossy().into_owned();
        let files = collect_files(dir);
        let fingerprint_dir = dir.join("fingerprint");
        let mut has_fingerprint = false;
        let mut runs_build_script = false;
        for path in files.iter().map(|entry| entry.path.as_path()) {
            if path.parent() != Some(fingerprint_dir.as_path()) {
                continue;
            }
            let is_json = path.extension().is_some_and(|extension| extension == "json");
            if !is_json {
                continue;
            }
            has_fingerprint = true;
            runs_build_script |= path
                .file_name()
                .is_some_and(|file| file.to_string_lossy().starts_with("run-build-script-"));
        }
        if !has_fingerprint {
            return None;
        }
        let version = registry_version(&files, &name);
        let files = files.into_iter().map(|entry| entry.stat).collect();
        Some(Self { name, version, dir: dir.to_path_buf(), files, runs_build_script })
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    pub fn dir(&self) -> &Path {
        self.dir.as_path()
    }

    pub fn label(&self) -> String {
        match &self.version {
            Some(version) => format!("{} {version}", self.name),
            None => self.name.clone(),
        }
    }

    pub fn artifacts_missing(&self) -> bool {
        if self.runs_build_script {
            return false;
        }
        let out = self.dir.join("out");
        out.is_dir() && !out_has_artifact(&out)
    }

    pub fn into_artifact(self) -> Artifact {
        let label = self.label();
        Artifact { label, paths: vec![self.dir], files: self.files }
    }
}

fn is_unit_hash(name: &str) -> bool {
    name.len() == UNIT_HASH_LEN && name.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn out_has_artifact(out: &Path) -> bool {
    let mut complete = true;
    let found = WalkDir::new(out).into_iter().any(|entry| match entry {
        Ok(entry) => {
            entry.file_type().is_file() && !entry.file_name().to_string_lossy().ends_with(".d")
        },
        Err(_) => {
            complete = false;
            false
        },
    });
    found || !complete
}

fn registry_version(files: &[FileEntry], name: &str) -> Option<String> {
    files
        .iter()
        .filter(|entry| entry.path.extension().is_some_and(|extension| extension == "d"))
        .find_map(|entry| version_from_dep_info(&fs::read_to_string(&entry.path).ok()?, name))
}

fn version_from_dep_info(dep_info: &str, name: &str) -> Option<String> {
    for token in dep_info.split_whitespace() {
        let components: Vec<&str> = Path::new(token.trim_end_matches(':'))
            .components()
            .filter_map(|component| match component {
                Component::Normal(segment) => segment.to_str(),
                _ => None,
            })
            .collect();
        for window in components.windows(REGISTRY_PATH_LEN) {
            if window[0] != "registry" || window[1] != "src" {
                continue;
            }
            let Some(rest) = window[3].strip_prefix(name) else {
                continue;
            };
            let Some(version) = rest.strip_prefix('-') else {
                continue;
            };
            if semver::Version::parse(version).is_ok() {
                return Some(version.to_owned());
            }
        }
    }
    None
}
