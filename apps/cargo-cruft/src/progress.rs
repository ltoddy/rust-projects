use std::cell::RefCell;
use std::io::IsTerminal;

use indicatif::{ProgressBar, ProgressStyle};

const BAR: &str = "{msg} {bar:32} {pos}/{len}";
const SPINNER: &str = "{msg} {spinner} {pos}";

#[derive(Debug)]
pub struct Progress {
    enabled: bool,
    bar: RefCell<ProgressBar>,
}

impl Default for Progress {
    fn default() -> Self {
        Self::new()
    }
}

impl Progress {
    pub fn new() -> Self {
        let enabled = std::io::stderr().is_terminal();
        Self { enabled, bar: RefCell::new(Self::create(enabled)) }
    }

    pub fn begin(&self, message: &str, total: u64) {
        let bar = self.bar.borrow();
        bar.set_style(if total == 0 { Self::style(SPINNER) } else { Self::style(BAR) });
        bar.set_length(total);
        bar.set_position(0);
        bar.set_message(message.to_owned());
        bar.tick();
    }

    pub fn advance(&self) {
        self.bar.borrow().inc(1);
    }

    pub fn clear(&self) {
        let mut bar = self.bar.borrow_mut();
        bar.finish_and_clear();
        *bar = Self::create(self.enabled);
    }

    fn create(enabled: bool) -> ProgressBar {
        if enabled { ProgressBar::new(0) } else { ProgressBar::hidden() }
    }

    fn style(template: &str) -> ProgressStyle {
        ProgressStyle::with_template(template).unwrap_or_else(|_| ProgressStyle::default_bar())
    }
}
