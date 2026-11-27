use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "rcloc", version, about = env!("CARGO_PKG_DESCRIPTION"))]
pub struct Args {
    #[arg(value_name = "PATH", help = "要统计的目录或文件，缺省为当前工作目录")]
    pub path: Option<PathBuf>,

    #[arg(long, value_name = "N", default_value_t = 20, help = "最多展示的行数，0 表示全部")]
    pub top: usize,

    #[arg(long, help = "按文件统计，而不是按语言")]
    pub by_file: bool,
}

impl Args {
    pub fn path(&self) -> PathBuf {
        match &self.path {
            Some(path) => path.clone(),
            None => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }
}
