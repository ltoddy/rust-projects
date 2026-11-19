use std::path::Path;
use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};

use crate::error::Error;

const TEMPLATE: &str = "{spinner} {pos} files {per_sec} {wide_msg}";
const TICK_INTERVAL: Duration = Duration::from_millis(100);

pub struct Progress {
    bar: ProgressBar,
}

impl Progress {
    pub fn new() -> Result<Self, Error> {
        let bar = ProgressBar::new_spinner();
        bar.set_style(ProgressStyle::with_template(TEMPLATE).map_err(|_| Error::template())?);
        bar.enable_steady_tick(TICK_INTERVAL);

        Ok(Self { bar })
    }

    pub fn report(&self, path: impl AsRef<Path>) {
        let path = path.as_ref();
        self.bar.set_message(path.display().to_string());
    }

    pub fn advance(&self) {
        self.bar.inc(1);
    }

    pub fn finish(&self, files: usize) {
        self.bar.finish_with_message(format!("{files} files"));
    }
}
