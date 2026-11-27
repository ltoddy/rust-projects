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

    #[error("非文本文件 {path}")]
    NotText { path: PathBuf },

    #[error("进度条模板无效")]
    Template,

    #[error("工作线程异常退出")]
    Thread,
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

    pub fn not_text(path: PathBuf) -> Self {
        Self::NotText { path }
    }

    pub fn template() -> Self {
        Self::Template
    }

    pub fn thread() -> Self {
        Self::Thread
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}
