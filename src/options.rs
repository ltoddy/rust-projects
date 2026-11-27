use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Output {
    Terminal,
    Markdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SortBy {
    Language,
    Files,
    Size,
    Blank,
    Comment,
    Code,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OrderBy {
    Asc,
    Desc,
}

#[derive(Debug, Parser)]
#[command(
    name = "rcloc",
    author,
    version,
    about = "Count, or compute differences of, lines of source code and comments.",
    after_help = "Contribute this project on Github: https://github.com/ltoddy/cloc-rs"
)]
pub struct Options {
    /// Format of the report
    #[arg(long = "output", value_enum, ignore_case = true, default_value = "terminal")]
    pub output: Output,

    /// Field used to sort the report
    #[arg(long = "sort-by", value_enum, ignore_case = true, default_value = "language")]
    pub sort_by: SortBy,

    /// Sort direction of the report
    #[arg(long = "order-by", value_enum, ignore_case = true, default_value = "asc")]
    pub order_by: OrderBy,

    /// File listing paths (one per line) to exclude from the report
    #[arg(long = "ignore-file")]
    pub ignore_file: Option<PathBuf>,

    /// Directory to analyse, defaults to the current directory
    #[arg(value_name = "path")]
    pub entry: Option<PathBuf>,
}
