use std::time::Duration;

use crate::calculator::Detail;
use crate::reporter::Report;
use crate::util::bytes_to_size;

const HEADER: [&str; 6] = ["Language", "files", "size", "blank", "comment", "code"];

pub fn pretty_print(report: &Report, elapsed: Duration) {
    println!("{:>12.4} secs", elapsed.as_secs_f64());
    println!(
        "┌───────────────────────────────────────────────────────────────────────────────────────┐"
    );
    print_row(&HEADER.map(String::from));
    println!(
        "├───────────────────────────────────────────────────────────────────────────────────────┤"
    );

    for detail in &report.sections {
        print_row(&cells(detail));
    }

    println!(
        "├───────────────────────────────────────────────────────────────────────────────────────┤"
    );
    print_row(&cells(&report.summary));
    println!(
        "└───────────────────────────────────────────────────────────────────────────────────────┘"
    );
}

pub fn print_markdown(report: &Report, elapsed: Duration) {
    println!("```");
    println!("{:>12.4} secs", elapsed.as_secs_f64());
    println!("```");
    println!();
    println!("| {} |", HEADER.join(" | "));
    println!("| {} |", ["---"; HEADER.len()].join(" | "));

    for detail in &report.sections {
        println!("| {} |", cells(detail).join(" | "));
    }

    println!("| {} |", cells(&report.summary).join(" | "));
}

fn cells(detail: &Detail) -> [String; 6] {
    [
        detail.language.to_string(),
        detail.files.to_string(),
        bytes_to_size(detail.bytes as f64),
        detail.blank.to_string(),
        detail.comment.to_string(),
        detail.code.to_string(),
    ]
}

fn print_row(cells: &[String; 6]) {
    println!(
        "│ {:<25}{:>12}{:>12}{:>12}{:>12}{:>12} │",
        cells[0], cells[1], cells[2], cells[3], cells[4], cells[5],
    );
}
