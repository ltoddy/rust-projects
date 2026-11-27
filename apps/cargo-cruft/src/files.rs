use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

#[cfg(unix)]
const DISK_BLOCK_SIZE: u64 = 512;
#[cfg(not(unix))]
const FALLBACK_BLOCK_SIZE: u64 = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileKey {
    device: u64,
    inode: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct FileStat {
    key: FileKey,
    bytes: u64,
    hard_links: u64,
}

impl FileStat {
    #[cfg_attr(unix, expect(unused_variables, reason = "只在没有 inode 的平台上用路径区分文件"))]
    pub fn of(path: &Path, metadata: &Metadata) -> Self {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;

            Self {
                key: FileKey { device: metadata.dev(), inode: metadata.ino() },
                bytes: metadata.blocks() * DISK_BLOCK_SIZE,
                hard_links: metadata.nlink(),
            }
        }
        #[cfg(not(unix))]
        {
            use std::hash::{Hash, Hasher};

            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            path.hash(&mut hasher);
            Self {
                key: FileKey { device: 0, inode: hasher.finish() },
                bytes: metadata.len().div_ceil(FALLBACK_BLOCK_SIZE) * FALLBACK_BLOCK_SIZE,
                hard_links: 1,
            }
        }
    }

    pub fn key(&self) -> FileKey {
        self.key
    }

    pub fn bytes(&self) -> u64 {
        self.bytes
    }

    pub fn hard_links(&self) -> u64 {
        self.hard_links
    }
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub stat: FileStat,
}

pub fn collect_files(root: &Path) -> Vec<FileEntry> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| {
            let metadata = fs::symlink_metadata(entry.path()).ok()?;
            Some(FileEntry {
                path: entry.path().to_path_buf(),
                stat: FileStat::of(entry.path(), &metadata),
            })
        })
        .collect()
}

pub fn entry_is_dir(entry: &fs::DirEntry) -> bool {
    entry.file_type().is_ok_and(|kind| kind.is_dir())
}

pub fn entry_name(entry: &fs::DirEntry) -> String {
    entry.file_name().to_string_lossy().into_owned()
}
