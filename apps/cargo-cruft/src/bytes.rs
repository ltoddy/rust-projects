use std::fmt::{self, Display};

const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];

#[derive(Debug, Clone, Copy)]
pub struct Bytes(pub u64);

impl Display for Bytes {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 < 1024 {
            return formatter.pad(&format!("{} B", self.0));
        }
        let mut value = self.0 as f64;
        let mut unit = 0;
        while value >= 1024.0 && unit < UNITS.len() - 1 {
            value /= 1024.0;
            unit += 1;
        }
        formatter.pad(&format!("{value:.1} {}", UNITS[unit]))
    }
}
