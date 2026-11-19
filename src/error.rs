use std::path::PathBuf;

use thiserror::Error;

#[derive(Error)]
pub enum Error {
    #[error("无法加载 tokenizer: {0}")]
    Tokenizer(#[from] Box<dyn std::error::Error + Send + Sync>),

    #[error("无法读取文件 {path}: {source}")]
    ReadFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("分词失败 {path}: {source}")]
    Encode {
        path: PathBuf,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("遍历目录失败: {0}")]
    Walk(#[from] ignore::Error),
}

impl Error {
    pub fn tokenizer(source: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self::Tokenizer(source)
    }

    pub fn read_file(path: PathBuf, source: std::io::Error) -> Self {
        Self::ReadFile { path, source }
    }

    pub fn encode(path: PathBuf, source: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self::Encode { path, source }
    }

    pub fn walk(source: ignore::Error) -> Self {
        Self::Walk(source)
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}
