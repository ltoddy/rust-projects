use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "tokstat", version, about = env!("CARGO_PKG_DESCRIPTION"))]
pub struct Args {
    #[arg(value_name = "PATH")]
    pub path: Option<PathBuf>,
}

impl Args {
    pub fn path(&self) -> PathBuf {
        match &self.path {
            Some(path) => path.clone(),
            None => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }
}
