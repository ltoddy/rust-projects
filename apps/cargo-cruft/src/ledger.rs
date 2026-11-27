use std::collections::HashMap;

use crate::files::{FileKey, FileStat};
use crate::report::Candidate;

#[derive(Debug)]
struct Inode {
    bytes: u64,
    hard_links: u64,
    claimed_links: u64,
    owner: usize,
}

impl Inode {
    fn new(stat: &FileStat, owner: usize) -> Self {
        Self { bytes: stat.bytes(), hard_links: stat.hard_links(), claimed_links: 0, owner }
    }
}

#[derive(Debug)]
pub struct Ledger {
    inodes: HashMap<FileKey, Inode>,
}

impl Ledger {
    pub fn new(candidates: &[Candidate]) -> Self {
        let mut inodes: HashMap<FileKey, Inode> = HashMap::new();
        for (owner, candidate) in candidates.iter().enumerate() {
            for stat in &candidate.files {
                let inode = inodes.entry(stat.key()).or_insert_with(|| Inode::new(stat, owner));
                inode.claimed_links += 1;
            }
        }
        Self { inodes }
    }

    pub fn claim(&mut self, stat: &FileStat) -> Option<usize> {
        let inode = self.inodes.get_mut(&stat.key())?;
        inode.claimed_links += 1;
        Some(inode.owner)
    }

    pub fn reclaimable(&self, candidates: usize) -> Vec<u64> {
        let mut bytes = vec![0; candidates];
        for inode in self.inodes.values() {
            if inode.claimed_links >= inode.hard_links {
                bytes[inode.owner] += inode.bytes;
            }
        }
        bytes
    }

    pub fn keys(&self) -> impl Iterator<Item = FileKey> + '_ {
        self.inodes.keys().copied()
    }

    pub fn total_bytes(&self) -> u64 {
        self.inodes.values().map(|inode| inode.bytes).sum()
    }
}
