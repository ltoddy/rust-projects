use std::fs;

use crate::files::entry_is_dir;
use crate::report::Artifact;
use crate::target::Profile;

const HASH_LEN: usize = 13;

pub fn scan(profile: &Profile) -> Vec<Artifact> {
    let Ok(entries) = fs::read_dir(profile.incremental_dir()) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            if !entry_is_dir(&entry) {
                return None;
            }
            let dir_name = entry.file_name();
            let crate_name = strip_hash(dir_name.to_str()?)?;
            Some(Artifact::from_directory(entry.path(), crate_name.to_owned()))
        })
        .collect()
}

fn strip_hash(dir_name: &str) -> Option<&str> {
    let (crate_name, hash) = dir_name.rsplit_once('-')?;
    if crate_name.is_empty()
        || hash.len() != HASH_LEN
        || !hash.bytes().all(|byte| byte.is_ascii_alphanumeric())
    {
        return None;
    }
    Some(crate_name)
}
