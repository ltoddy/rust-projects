//! Throughput of the append path, plus the cost of the codec on its own.
//!
//! Run with `cargo bench`.
//!
//! File numbers are **page-cache numbers**: no `sync()` is issued, so they say
//! nothing about disk throughput. What they are good for is the ratio between
//! the rows: buffered appending versus flushing every record, and our framing
//! versus calling `serde_json` directly.
//!
//! Append benchmarks keep one writer (and one file handle) open for the whole
//! run, so a sample measures appends only rather than file creation.
//!
#![allow(clippy::unwrap_used)]

use std::path::PathBuf;
use std::sync::OnceLock;

use bytes::BytesMut;
use divan::{Bencher, black_box};
use serde::{Deserialize, Serialize};
use tokio::runtime::{Builder, Runtime};
use tokio::sync::Mutex;
use tokio_jsonl::{JsonlCodec, JsonlHandle, JsonlWriter, spawn_writer};
use tokio_util::codec::{Decoder, Encoder};

/// Records per benchmark iteration.
const RECORDS: u64 = 2_000;

#[derive(Debug, Serialize, Deserialize)]
struct Record {
    ts: u64,
    level: String,
    kind: String,
    value: f64,
    message: String,
}

fn record(i: u64) -> Record {
    Record {
        ts: 1_700_000_000 + i,
        level: "info".to_owned(),
        kind: "request".to_owned(),
        value: i as f64 / 7.0,
        message: "GET /v1/things 200 in 4ms".to_owned(),
    }
}

fn ids() -> Vec<u64> {
    (0..RECORDS).collect()
}

fn runtime() -> &'static Runtime {
    static RT: OnceLock<Runtime> = OnceLock::new();
    RT.get_or_init(|| Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap())
}

/// A file inside a temp dir that lives for the whole benchmark run.
fn bench_file(name: &str) -> PathBuf {
    static DIR: OnceLock<tempfile::TempDir> = OnceLock::new();
    DIR.get_or_init(|| tempfile::tempdir().unwrap()).path().join(name)
}

/// Writers that live for the whole run, so that opening the file is not part
/// of the measurement.
macro_rules! shared_writer {
    ($name:ident, $file:literal) => {
        fn $name() -> &'static Mutex<JsonlWriter<tokio::fs::File, Record>> {
            static WRITER: OnceLock<Mutex<JsonlWriter<tokio::fs::File, Record>>> = OnceLock::new();
            WRITER.get_or_init(|| {
                let writer =
                    runtime().block_on(JsonlWriter::<_, Record>::open(bench_file($file))).unwrap();
                Mutex::new(writer)
            })
        }
    };
}

shared_writer!(buffered_writer, "buffered.jsonl");
shared_writer!(flush_each_writer, "flush_each.jsonl");

/// The encoded form of [`RECORDS`] records, for the decode benchmarks.
fn encoded() -> &'static [u8] {
    static BLOB: OnceLock<Vec<u8>> = OnceLock::new();
    BLOB.get_or_init(|| {
        let mut codec = JsonlCodec::<Record>::new();
        let mut buf = BytesMut::new();
        for id in ids() {
            codec.encode(&record(id), &mut buf).unwrap();
        }
        buf.to_vec()
    })
}

/// Serialization alone: no I/O, no buffers to spill.
#[divan::bench(items_count = RECORDS)]
fn encode(bencher: Bencher) {
    let records: Vec<Record> = ids().into_iter().map(record).collect();
    let mut codec = JsonlCodec::<Record>::new();
    let mut buf = BytesMut::with_capacity(1 << 20);

    bencher.bench_local(|| {
        for rec in &records {
            codec.encode(rec, &mut buf).unwrap();
        }
        black_box(&buf);
        buf.clear();
    });
}

/// The intended way to append: batch into the buffer, spill at the threshold.
///
/// The writer is opened once for the whole run, so the timed region is exactly
/// "append [`RECORDS`] records and flush".
#[divan::bench(items_count = RECORDS)]
fn append_buffered(bencher: Bencher) {
    let ids = ids();
    let writer = buffered_writer();

    bencher.bench_local(|| {
        runtime().block_on(async {
            let mut writer = writer.lock().await;
            for id in &ids {
                writer.write(&record(*id)).await.unwrap();
            }
            writer.flush().await.unwrap();
        })
    });
}

/// The anti-pattern this crate exists to avoid: one flush per record.
#[divan::bench(items_count = RECORDS)]
fn append_flush_each(bencher: Bencher) {
    let ids = ids();
    let writer = flush_each_writer();

    bencher.bench_local(|| {
        runtime().block_on(async {
            let mut writer = writer.lock().await;
            for id in &ids {
                writer.write(&record(*id)).await.unwrap();
                writer.flush().await.unwrap();
            }
        })
    });
}

fn handle() -> &'static JsonlHandle<Record> {
    static HANDLE: OnceLock<JsonlHandle<Record>> = OnceLock::new();
    HANDLE.get_or_init(|| {
        runtime().block_on(spawn_writer::<Record>(bench_file("handle.jsonl"))).unwrap()
    })
}

/// What the cloneable handle costs: a channel hop per record.
///
/// The API takes records by value, so each iteration also builds its records;
/// [`append_buffered`] does the same, which makes the delta between the two
/// rows the channel itself.
#[divan::bench(items_count = RECORDS)]
fn append_via_handle(bencher: Bencher) {
    let ids = ids();
    let handle = handle();

    bencher.bench_local(|| {
        runtime().block_on(async {
            for id in &ids {
                handle.write(record(*id)).await.unwrap();
            }
            handle.flush().await.unwrap();
        })
    });
}

/// Decoding through the codec. The input buffer is generated outside the timed
/// region, so this measures decoding only.
#[divan::bench(items_count = RECORDS)]
fn decode(bencher: Bencher) {
    let blob = encoded();
    let mut codec = JsonlCodec::<Record>::new();

    bencher.with_inputs(|| BytesMut::from(blob)).bench_local_values(|mut buf| {
        let mut count = 0;
        while let Some(rec) = codec.decode(&mut buf).unwrap() {
            black_box(&rec);
            count += 1;
        }
        assert_eq!(count, RECORDS);
    });
}

/// Baseline for [`decode`]: the same payload straight through `serde_json`.
/// The difference between the two rows is what the line framing costs.
#[divan::bench(items_count = RECORDS)]
fn decode_serde_baseline(bencher: Bencher) {
    let text = std::str::from_utf8(encoded()).unwrap();

    bencher.bench_local(|| {
        for line in text.lines() {
            black_box(serde_json::from_str::<Record>(line).unwrap());
        }
    });
}

fn main() {
    divan::main();
}
