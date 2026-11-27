use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Error)]
pub enum Error {
    #[error("无法读取文件 {path}: {source}")]
    ReadFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("不支持的文件类型 {path}")]
    Unsupported { path: PathBuf },

    #[error("非文本文件 {path}")]
    NotText { path: PathBuf },

    #[error("路径不存在 {path}")]
    Missing { path: PathBuf },

    #[error("工作线程异常退出")]
    Thread,

    #[error("进度条模板无效")]
    Template,
}

impl Error {
    pub fn read_file(path: &Path, source: std::io::Error) -> Self {
        Self::ReadFile { path: path.to_path_buf(), source }
    }

    pub fn unsupported(path: &Path) -> Self {
        Self::Unsupported { path: path.to_path_buf() }
    }

    pub fn not_text(path: &Path) -> Self {
        Self::NotText { path: path.to_path_buf() }
    }

    pub fn missing(path: &Path) -> Self {
        Self::Missing { path: path.to_path_buf() }
    }

    pub fn thread() -> Self {
        Self::Thread
    }

    pub fn template() -> Self {
        Self::Template
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}
