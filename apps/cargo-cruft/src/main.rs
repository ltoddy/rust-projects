mod bytes;
mod cleaner;
mod files;
mod incremental;
mod ledger;
mod lockfile;
mod progress;
mod render;
mod report;
mod scanner;
mod target;
mod unit;

use std::env;

use anyhow::{Context, Result, ensure};

use crate::bytes::Bytes;
use crate::cleaner::Cleaner;
use crate::lockfile::LockSet;
use crate::progress::Progress;
use crate::scanner::Scanner;
use crate::target::{TargetDir, TargetGuard};

fn main() -> Result<()> {
    let cwd = env::current_dir().context("cannot read the current directory")?;
    let target_root = cwd.join("target");
    let Some(target) = TargetDir::open(&target_root) else {
        println!("{} does not exist, nothing to clean", target_root.display());
        return Ok(());
    };

    let lock = LockSet::load(&cwd.join("Cargo.lock"));
    if lock.is_none() {
        eprintln!(
            "warning: no readable Cargo.lock found, units of packages that left the lock are not reported"
        );
    }

    let _guard = TargetGuard::acquire(target.profiles())?;
    let progress = Progress::new();
    let report = Scanner::new(&target, lock.as_ref(), &progress).scan();
    print!("{}", render::report(&report));
    if report.candidates.is_empty() {
        return Ok(());
    }

    let cleanup = Cleaner::new(&report.candidates, &progress).run();
    let text = render::cleanup(&cleanup, Bytes(report.reclaimable()));
    print!("{}", text.summary);
    eprint!("{}", text.details);
    ensure!(cleanup.errors.is_empty(), "{} paths could not be deleted", cleanup.errors.len());
    Ok(())
}
