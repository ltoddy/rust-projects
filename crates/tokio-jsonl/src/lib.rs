// A crate built with `default-features = false` is missing the items the docs
// link to; that is expected, so do not warn about the resulting dead links.
#![cfg_attr(
    any(not(feature = "fs"), not(feature = "background")),
    allow(rustdoc::broken_intra_doc_links)
)]

//! JSON Lines ([jsonl](https://jsonlines.org/)) for the tokio ecosystem.
//!
//! The crate is built around [`tokio_util::codec`], so everything is a thin
//! layer over `FramedRead` / `FramedWrite` and composes with the rest of the
//! ecosystem. On top of that it adds what appending to a `.jsonl` file actually
//! needs: buffered writes, append-mode files, interval flushing, and decode
//! errors that point at a line number.
//!
//! # Appending records
//!
//! ```no_run
//! use serde::Serialize;
//!
//! #[derive(Serialize)]
//! struct Event {
//!     level: &'static str,
//!     msg: &'static str,
//! }
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # #[cfg(feature = "fs")]
//! # {
//! let mut writer = tokio_jsonl::JsonlWriter::<_, Event>::open("events.jsonl").await?;
//! writer.write(&Event { level: "info", msg: "started" }).await?;
//! writer.close().await?;
//! # }
//! # Ok(())
//! # }
//! ```
//!
//! [`JsonlWriter::open`] opens in append mode, so each run adds to the file
//! instead of replacing it, and records are buffered (64 KiB by default) so a
//! hot loop does not turn every record into a syscall.
//!
//! # Appending from several tasks
//!
//! ```no_run
//! use serde::Serialize;
//!
//! #[derive(Serialize, Clone)]
//! struct Metric {
//!     name: String,
//!     value: f64,
//! }
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # #[cfg(feature = "background")]
//! # {
//! let handle = tokio_jsonl::spawn_writer::<Metric>("metrics.jsonl").await?;
//! let worker = handle.clone();
//! tokio::spawn(async move {
//!     worker
//!         .write(Metric { name: "cpu".into(), value: 0.5 })
//!         .await
//!         .unwrap();
//! });
//! handle.flush().await?;
//! # }
//! # Ok(())
//! # }
//! ```
//!
//! The background task owns the file, flushes every second regardless of
//! traffic, and applies backpressure through a bounded queue.
//!
//! # One-shot helpers
//!
//! For small files, [`write_all`], [`append_all`] and [`read_all`] mirror
//! `std::fs`, so a script does not need to touch the reader and writer types:
//!
//! ```no_run
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Serialize, Deserialize)]
//! struct Event {
//!     level: String,
//! }
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # #[cfg(feature = "fs")]
//! # {
//! tokio_jsonl::write_all("events.jsonl", vec![Event { level: "info".into() }]).await?;
//! tokio_jsonl::append_all("events.jsonl", vec![Event { level: "warn".into() }]).await?;
//!
//! let all: Vec<Event> = tokio_jsonl::read_all("events.jsonl").await?;
//! assert_eq!(all.len(), 2);
//! # }
//! # Ok(())
//! # }
//! ```
//!
//! They read and write everything at once and are all-or-nothing: a single
//! malformed line fails [`read_all`].
//!
//! # Reading
//!
//! ```no_run
//! use serde::Deserialize;
//!
//! #[derive(Deserialize)]
//! struct Event {
//!     level: String,
//! }
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # #[cfg(feature = "fs")]
//! # {
//! let mut reader = tokio_jsonl::JsonlReader::<_, Event>::open("events.jsonl").await?;
//! while let Some(event) = reader.next().await {
//!     let event = event?;
//!     println!("{}", event.level);
//! }
//! # }
//! # Ok(())
//! # }
//! ```
//!
//! # Format notes
//!
//! When decoding, blank lines are skipped, `\r\n` is accepted, and a final line
//! without a trailing `\n` is still returned. There is no line length limit: a
//! record is buffered until its terminator arrives, so a truncated file without
//! any newline is held in memory as one long line. A bad line surfaces once as
//! [`Error::Json`] carrying its line number, and the reader ends after that
//! error, exactly like any other `FramedRead`.
//!
//! # Cargo features
//!
//! Everything is on by default. The codec and the in-memory reader / writer
//! need no features at all, so `default-features = false` keeps only
//! `JsonlCodec`, [`JsonlReader`] and [`JsonlWriter`] over any `AsyncRead` /
//! `AsyncWrite`.
//!
//! - `fs`: path-based constructors (`JsonlReader::open`, `JsonlWriter::open`,
//!   `create`, `sync`) and the one-shot [`read_all`] / [`write_all`] /
//!   [`append_all`] helpers. Pulls in `tokio/fs`.
//! - `background`: [`spawn_writer`], `spawn_writer_with`, [`WriterConfig`] and
//!   [`JsonlHandle`]. Implies `fs`, and adds `tokio/rt`, `tokio/sync`,
//!   `tokio/time` and `tokio/macros`.

#[cfg(feature = "background")]
mod background;
mod codec;
#[cfg(feature = "fs")]
mod fs;
mod reader;
mod writer;

#[cfg(feature = "background")]
pub use background::{JsonlHandle, WriterConfig, spawn_writer, spawn_writer_with};
pub use codec::{Error, JsonlCodec};
#[cfg(feature = "fs")]
pub use fs::{append_all, read_all, write_all};
pub use reader::{DEFAULT_READ_CAPACITY, JsonlReader};
pub use writer::{DEFAULT_FLUSH_THRESHOLD, JsonlWriter};
