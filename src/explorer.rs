use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};

pub struct Explorer {
    ignore_list: Option<Vec<PathBuf>>,
    sender: SyncSender<PathBuf>,
}

impl Explorer {
    pub fn new(ignore_list: Option<Vec<PathBuf>>) -> (Self, Receiver<PathBuf>) {
        let (sender, receiver) = sync_channel::<PathBuf>(1024);
        let explorer = Self { ignore_list, sender };
        (explorer, receiver)
    }

    pub fn walk_directory(&mut self, entry: &Path) {
        if entry.is_file() {
            if self.is_not_ignore_file(entry) {
                let _ = self.sender.send(entry.to_path_buf());
            }
        } else if entry.is_dir()
            && let Ok(entries) = fs::read_dir(entry)
        {
            for entry in entries.flatten() {
                self.walk_directory(&entry.path());
            }
        }
    }

    fn is_not_ignore_file(&self, filename: &Path) -> bool {
        match &self.ignore_list {
            Some(ignore_list) => ignore_list.iter().all(|path| !filename.starts_with(path)),
            None => true,
        }
    }
}
