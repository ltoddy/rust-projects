use std::collections::HashSet;
use std::fs;
use std::path::Path;

use toml::Table;

#[derive(Debug)]
pub struct LockSet {
    names: HashSet<String>,
    versions: HashSet<(String, String)>,
}

impl LockSet {
    pub fn load(path: &Path) -> Option<Self> {
        Self::parse(&fs::read_to_string(path).ok()?)
    }

    pub fn parse(text: &str) -> Option<Self> {
        let lock: Table = text.parse().ok()?;
        let packages = lock.get("package")?.as_array()?;
        let mut names = HashSet::with_capacity(packages.len());
        let mut versions = HashSet::with_capacity(packages.len());
        for package in packages {
            let Some(package) = package.as_table() else {
                continue;
            };
            let Some(name) = package.get("name").and_then(|value| value.as_str()) else {
                continue;
            };
            let name = normalize_name(name);
            if let Some(version) = package.get("version").and_then(|value| value.as_str()) {
                versions.insert((name.clone(), version.to_owned()));
            }
            names.insert(name);
        }
        Some(Self { names, versions })
    }

    pub fn contains_package(&self, name: &str, version: Option<&str>) -> bool {
        let name = normalize_name(name);
        match version {
            Some(version) => self.versions.contains(&(name, version.to_owned())),
            None => self.names.contains(&name),
        }
    }
}

fn normalize_name(name: &str) -> String {
    name.replace('-', "_")
}
