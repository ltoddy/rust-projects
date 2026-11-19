mod args;

use clap::Parser;

use crate::args::Args;

fn main() {
    let args = Args::parse();
    let root = args.path();
    println!("{}", root.display());
}
