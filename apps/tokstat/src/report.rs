use std::fmt;
use std::path::{Path, PathBuf};

const TOKENS_HEADER: &str = "TOKENS";
const SHARE_HEADER: &str = "SHARE";
const FILE_HEADER: &str = "FILE";
const SHARE_WIDTH: usize = 6;
const COLUMN_GAP: usize = 2;

#[derive(Clone, Copy)]
pub struct Tokens(usize);

impl Tokens {
    pub fn new(value: usize) -> Self {
        Self(value)
    }

    pub fn value(self) -> usize {
        self.0
    }

    pub fn share_of(self, total: Self) -> f64 {
        match total.value() {
            0 => 0.0,
            total => self.0 as f64 * 100.0 / total as f64,
        }
    }

    fn width(self) -> usize {
        self.to_string().len()
    }
}

impl fmt::Display for Tokens {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let digits = self.0.to_string();
        let mut formatted = String::with_capacity(digits.len() + digits.len() / 3);

        for (index, digit) in digits.chars().enumerate() {
            if index > 0 && (digits.len() - index) % 3 == 0 {
                formatted.push(',');
            }
            formatted.push(digit);
        }

        f.pad(&formatted)
    }
}

pub struct Stat {
    path: PathBuf,
    tokens: Tokens,
}

impl Stat {
    pub fn new(path: impl Into<PathBuf>, tokens: Tokens) -> Self {
        let path = path.into();
        Self { path, tokens }
    }

    pub fn tokens(&self) -> Tokens {
        self.tokens
    }

    pub fn relative_to(&self, root: impl AsRef<Path>) -> String {
        let root = root.as_ref();
        match self.path.strip_prefix(root) {
            Ok(stripped) if !stripped.as_os_str().is_empty() => stripped.display().to_string(),
            _ => self.path.display().to_string(),
        }
    }
}

pub struct ScanResult {
    stats: Vec<Stat>,
    skipped: usize,
    dir_errors: usize,
}

impl ScanResult {
    pub fn new(stats: Vec<Stat>, skipped: usize, dir_errors: usize) -> Self {
        Self {
            stats,
            skipped,
            dir_errors,
        }
    }

    pub fn absorb(&mut self, stats: Vec<Stat>, skipped: usize) {
        self.stats.extend(stats);
        self.skipped += skipped;
    }

    pub fn with_dir_errors(mut self, dir_errors: usize) -> Self {
        self.dir_errors = dir_errors;
        self
    }

    pub fn stats(&self) -> &[Stat] {
        &self.stats
    }

    pub fn files(&self) -> usize {
        self.stats.len()
    }

    pub fn skipped(&self) -> usize {
        self.skipped
    }

    pub fn dir_errors(&self) -> usize {
        self.dir_errors
    }
}

pub struct Report {
    root: PathBuf,
    scan: ScanResult,
}

impl Report {
    pub fn new(root: impl Into<PathBuf>, scan: ScanResult) -> Self {
        let root = root.into();
        Self { root, scan }
    }

    pub fn render(&self, top: usize) {
        let mut ordered = self.scan.stats().iter().collect::<Vec<_>>();
        ordered.sort_by_key(|stat| std::cmp::Reverse(stat.tokens().value()));

        let total = Tokens::new(ordered.iter().map(|stat| stat.tokens().value()).sum());
        let files = ordered.len();
        let shown = match top {
            0 => files,
            top => files.min(top),
        };

        let rows = ordered
            .iter()
            .take(shown)
            .map(|stat| {
                Row::new(
                    stat.relative_to(&self.root),
                    stat.tokens(),
                    stat.tokens().share_of(total),
                )
            })
            .collect::<Vec<_>>();

        let tokens_width = rows
            .iter()
            .map(|row| row.tokens.width())
            .chain(std::iter::once(total.width()))
            .max()
            .unwrap_or(0)
            .max(TOKENS_HEADER.len());
        let percent_width = SHARE_WIDTH - 1;
        let gap = " ".repeat(COLUMN_GAP);

        println!(
            "{:>tokens_width$}{gap}{:>share_width$}{gap}{}",
            TOKENS_HEADER,
            SHARE_HEADER,
            FILE_HEADER,
            share_width = SHARE_WIDTH
        );

        for row in &rows {
            println!(
                "{:>tokens_width$}{gap}{:>percent_width$.1}%{gap}{}",
                row.tokens, row.share, row.name
            );
        }

        if shown < files {
            println!("… 还有 {} 个文件", files - shown);
        }

        println!(
            "{}",
            "-".repeat(tokens_width + COLUMN_GAP + SHARE_WIDTH + COLUMN_GAP + FILE_HEADER.len())
        );

        let mut summary = format!("{files} files, {} skipped", self.scan.skipped());
        if self.scan.dir_errors() > 0 {
            summary.push_str(&format!(", {} dir errors", self.scan.dir_errors()));
        }

        println!(
            "{:>tokens_width$}{gap}{:>percent_width$.1}%{gap}{summary}",
            total,
            total.share_of(total)
        );
    }
}

struct Row {
    name: String,
    tokens: Tokens,
    share: f64,
}

impl Row {
    fn new(name: String, tokens: Tokens, share: f64) -> Self {
        Self {
            name,
            tokens,
            share,
        }
    }
}
