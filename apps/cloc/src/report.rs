use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::args::Args;
use crate::count::Detail;

const NAME_HEADER: &str = "LANGUAGE";
const FILE_HEADER: &str = "FILE";
const FILES_HEADER: &str = "FILES";
const SIZE_HEADER: &str = "SIZE";
const BLANK_HEADER: &str = "BLANK";
const COMMENT_HEADER: &str = "COMMENT";
const CODE_HEADER: &str = "CODE";
const SHARE_HEADER: &str = "SHARE";
const TOTAL_NAME: &str = "Total";
const COLUMN_GAP: usize = 2;
const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Count(usize);

impl Count {
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
}

impl fmt::Display for Count {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let digits = self.0.to_string();
        let mut formatted = String::with_capacity(digits.len() + digits.len() / 3);

        for (index, digit) in digits.chars().enumerate() {
            if index > 0 && (digits.len() - index).is_multiple_of(3) {
                formatted.push(',');
            }
            formatted.push(digit);
        }

        f.pad(&formatted)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Size(u64);

impl Size {
    pub fn new(bytes: u64) -> Self {
        Self(bytes)
    }

    pub fn value(self) -> u64 {
        self.0
    }
}

impl fmt::Display for Size {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 < 1024 {
            return f.pad(&format!("{} B", Count::new(self.0 as usize)));
        }

        let mut value = self.0 as f64;
        let mut unit = 0;

        while value >= 1024.0 && unit < UNITS.len() - 1 {
            value /= 1024.0;
            unit += 1;
        }

        f.pad(&format!("{value:.1} {}", UNITS[unit]))
    }
}

pub struct Stat {
    path: PathBuf,
    detail: Detail,
}

impl Stat {
    pub fn new(path: PathBuf, detail: Detail) -> Self {
        Self { path, detail }
    }

    pub fn detail(&self) -> Detail {
        self.detail
    }

    pub fn relative_to(&self, root: &Path) -> String {
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
        Self { stats, skipped, dir_errors }
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
        Self { root: root.into(), scan }
    }

    pub fn render(&self, args: &Args) {
        Self::terminal(&self.view(args));
    }

    fn view(&self, args: &Args) -> View {
        let mut rows = self.rows(args.by_file);
        rows.sort_by(|prev, next| {
            next.code.cmp(&prev.code).then_with(|| prev.name.cmp(&next.name))
        });

        let total = Row::total(&rows);
        let lines = rows.len();
        let shown = match args.top {
            0 => lines,
            top => lines.min(top),
        };
        let code = total.code;

        View {
            header: Self::header(args.by_file),
            rows: rows[..shown].iter().map(|row| row.cells(args.by_file, code)).collect(),
            total: total.cells(args.by_file, code),
            hidden: lines - shown,
            files: self.scan.files(),
            skipped: self.scan.skipped(),
            dir_errors: self.scan.dir_errors(),
        }
    }

    fn rows(&self, by_file: bool) -> Vec<Row> {
        if by_file {
            return self
                .scan
                .stats()
                .iter()
                .map(|stat| Row::of(stat.relative_to(&self.root), &stat.detail()))
                .collect();
        }

        let mut grouped = HashMap::<&'static str, Detail>::new();

        for stat in self.scan.stats() {
            let detail = stat.detail();
            grouped
                .entry(detail.language())
                .and_modify(|total| *total += &detail)
                .or_insert(detail);
        }

        grouped.values().map(|detail| Row::of(detail.language().to_string(), detail)).collect()
    }

    fn header(by_file: bool) -> Vec<&'static str> {
        let mut header = Vec::with_capacity(7);
        header.push(if by_file { FILE_HEADER } else { NAME_HEADER });

        if !by_file {
            header.push(FILES_HEADER);
        }

        header.extend([SIZE_HEADER, BLANK_HEADER, COMMENT_HEADER, CODE_HEADER, SHARE_HEADER]);
        header
    }

    fn terminal(view: &View) {
        let header = view.header.iter().map(ToString::to_string).collect::<Vec<_>>();
        let widths = Self::widths(&header, view);

        println!("{}", Self::line(&header, &widths));

        for row in &view.rows {
            println!("{}", Self::line(row, &widths));
        }

        if view.hidden > 0 {
            println!("… 还有 {} 行", view.hidden);
        }

        println!("{}", "-".repeat(Self::line_width(&widths)));
        println!("{}", Self::line(&view.total, &widths));
        println!("{}", Self::summary(view));
    }

    fn widths(header: &[String], view: &View) -> Vec<usize> {
        header
            .iter()
            .enumerate()
            .map(|(index, title)| {
                let rows =
                    view.rows.iter().map(|row| row[index].chars().count()).max().unwrap_or(0);

                title.chars().count().max(rows).max(view.total[index].chars().count())
            })
            .collect()
    }

    fn line(cells: &[String], widths: &[usize]) -> String {
        let mut line = String::new();

        for (index, (cell, width)) in cells.iter().zip(widths).enumerate() {
            if index > 0 {
                line.push_str(&" ".repeat(COLUMN_GAP));
            }

            if index == 0 {
                line.push_str(&format!("{cell:<width$}"));
            } else {
                line.push_str(&format!("{cell:>width$}"));
            }
        }

        line.trim_end().to_string()
    }

    fn line_width(widths: &[usize]) -> usize {
        widths.iter().sum::<usize>() + COLUMN_GAP * widths.len().saturating_sub(1)
    }

    fn summary(view: &View) -> String {
        let mut summary = format!("{} files, {} skipped", view.files, view.skipped);

        if view.dir_errors > 0 {
            summary.push_str(&format!(", {} dir errors", view.dir_errors));
        }

        summary
    }
}

struct View {
    header: Vec<&'static str>,
    rows: Vec<Vec<String>>,
    total: Vec<String>,
    hidden: usize,
    files: usize,
    skipped: usize,
    dir_errors: usize,
}

pub struct Row {
    name: String,
    files: Count,
    size: Size,
    blank: Count,
    comment: Count,
    code: Count,
}

impl Row {
    pub fn of(name: String, detail: &Detail) -> Self {
        Self {
            name,
            files: Count::new(detail.files()),
            size: Size::new(detail.bytes()),
            blank: Count::new(detail.blank()),
            comment: Count::new(detail.comment()),
            code: Count::new(detail.code()),
        }
    }

    pub fn total(rows: &[Row]) -> Self {
        Self {
            name: String::from(TOTAL_NAME),
            files: Count::new(rows.iter().map(|row| row.files.value()).sum()),
            size: Size::new(rows.iter().map(|row| row.size.value()).sum()),
            blank: Count::new(rows.iter().map(|row| row.blank.value()).sum()),
            comment: Count::new(rows.iter().map(|row| row.comment.value()).sum()),
            code: Count::new(rows.iter().map(|row| row.code.value()).sum()),
        }
    }

    fn cells(&self, by_file: bool, code: Count) -> Vec<String> {
        let mut cells = Vec::with_capacity(7);
        cells.push(self.name.clone());

        if !by_file {
            cells.push(self.files.to_string());
        }

        cells.push(self.size.to_string());
        cells.push(self.blank.to_string());
        cells.push(self.comment.to_string());
        cells.push(self.code.to_string());
        cells.push(format!("{:.1}%", self.code.share_of(code)));
        cells
    }
}
