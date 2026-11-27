use std::collections::HashMap;
use std::sync::mpsc::Receiver;

use crate::calculator::Detail;

pub struct Reporter {
    receiver: Receiver<Detail>,
}

impl Reporter {
    pub fn new(receiver: Receiver<Detail>) -> Self {
        Self { receiver }
    }

    pub fn research(&self) -> Report {
        let mut kinds = HashMap::<&'static str, Detail>::new();
        let mut summary =
            Detail { language: "Sum", files: 0, bytes: 0, blank: 0, comment: 0, code: 0 };

        for detail in &self.receiver {
            summary += &detail;

            kinds.entry(detail.language).and_modify(|acc| *acc += &detail).or_insert(detail);
        }

        Report { sections: kinds.into_values().collect(), summary }
    }
}

#[derive(Debug)]
pub struct Report {
    pub sections: Vec<Detail>,
    pub summary: Detail,
}
