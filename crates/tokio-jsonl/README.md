# tokio-jsonl

Buffered, append-oriented [JSON Lines](https://jsonlines.org/) for the tokio ecosystem.

Reading and writing are thin layers over [`tokio_util::codec`](https://docs.rs/tokio-util/latest/tokio_util/codec/), so
they compose with `FramedRead` / `FramedWrite` and the rest of the ecosystem. On top of that, this crate adds what
appending to a `.jsonl` file actually needs: append-mode files, batched writes, interval flushing, backpressure, and
decode errors that tell you *which line* is broken.

```toml
[dependencies]
tokio-jsonl = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
serde = { version = "1", features = ["derive"] }
```

Requires Rust 1.85+ (edition 2024).

## Cargo features

Both features are on by default. The codec and the in-memory reader / writer need neither, so
`default-features = false` still gives you `JsonlCodec`, `JsonlReader` and `JsonlWriter` over any `AsyncRead` /
`AsyncWrite`.

| Feature      | Adds                                                                                                  | Pulls in                                               |
|--------------|-------------------------------------------------------------------------------------------------------|--------------------------------------------------------|
| `fs`         | `JsonlReader::open`, `JsonlWriter::open` / `create` / `sync`, `read_all` / `write_all` / `append_all` | `tokio/fs`                                             |
| `background` | `spawn_writer`, `spawn_writer_with`, `WriterConfig`, `JsonlHandle` (implies `fs`)                     | `tokio/rt`, `tokio/sync`, `tokio/time`, `tokio/macros` |

```toml
tokio-jsonl = { version = "0.1", default-features = false }
tokio-jsonl = { version = "0.1", default-features = false, features = ["fs"] }
```

The first line is the in-memory-only build, the second adds the file helpers.

## Appending to a file

`JsonlWriter::open` opens in append mode: an existing file is extended, a missing one is created. Records are buffered,
so a hot loop does not turn every record into a syscall.

```rust
use serde::Serialize;

#[derive(Serialize)]
struct Event<'a> {
    level: &'a str,
    msg: &'a str,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut writer = tokio_jsonl::JsonlWriter::<_, Event<'_>>::open("events.jsonl").await?;

    for _ in 0..1_000 {
        writer.write(&Event { level: "info", msg: "tick" }).await?;
    }

    writer.close().await?; // flush + shutdown, consumes the writer
    Ok(())
}
```

Use `JsonlWriter::create` instead if you want to truncate the file, or `JsonlWriter::new(any_async_write)` to write to a
socket, a pipe, or an in-memory buffer.

## Appending from many tasks

`spawn_writer` (the `background` feature) hands the file to a background task and gives you a cheap, cloneable handle.
The task flushes on a timer and applies backpressure through a bounded queue.

```rust
use serde::Serialize;

#[derive(Serialize)]
struct Metric {
    name: String,
    value: f64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let handle = tokio_jsonl::spawn_writer::<Metric>("metrics.jsonl").await?;

    let mut tasks = Vec::new();
    for _ in 0..8 {
        let handle = handle.clone();
        tasks.push(tokio::spawn(async move {
            for _ in 0..1_000 {
                handle
                    .write(Metric { name: "cpu".into(), value: 0.5 })
                    .await
                    .unwrap();
            }
        }));
    }
    for task in tasks {
        task.await?;
    }

    handle.flush().await?; // everything queued so far is on disk
    Ok(())
}
```

Tune it with `spawn_writer_with(path, WriterConfig::new().flush_interval(..))`.

## One-shot helpers

For small files the crate mirrors `std::fs`, so a script never has to name the reader or writer types:

```rust
let events = vec![Event { level: "info".into() }];

tokio_jsonl::write_all("events.jsonl", events).await?; // truncates the file
tokio_jsonl::append_all("events.jsonl", vec![Event { level: "warn".into() }]).await?;

let events: Vec<Event> = tokio_jsonl::read_all("events.jsonl").await?;
```

All three return the crate's `Error` and are all-or-nothing: a single malformed line fails `read_all` with its line
number, and nothing is silently skipped. `read_all` holds the whole file in memory — use `JsonlReader` for files that do
not fit, and `JsonlWriter` / `spawn_writer` for a long-lived appender rather than repeated `append_all` calls.

## Reading

```rust
use serde::Deserialize;

#[derive(Deserialize)]
struct Event {
    level: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = tokio_jsonl::JsonlReader::<_, Event>::open("events.jsonl").await?;

    while let Some(event) = reader.next().await {
        println!("{}", event?.level);
    }

    Ok(())
}
```

Need `Stream` combinators? `reader.into_inner()` returns the underlying `FramedRead`.

## Behavior worth knowing

### Durability

- `write()` returns once the record is encoded into the in-memory buffer — it is not on disk yet.
- The buffer is written out when it reaches the flush threshold (64 KiB by default), on `flush()`, and on `close()`.
- `flush()` hands the buffer to the OS; `sync()` (file writers only) additionally `fsync`s, which is what survives a
  machine crash.
- **Dropping a writer without flushing loses whatever was still buffered.** Call `close()`, or use `spawn_writer`, whose
  task flushes every second and again when the last handle goes away.
- Append mode uses `O_APPEND`, so several processes can append to the same file without clobbering each other. Each
  batch is written with whole lines, so records do not get spliced together; on network filesystems, treat this as best
  effort rather than a guarantee.

### Decoding

- Blank and whitespace-only lines are skipped; `\r\n` is accepted alongside `\n`.
- A final line without a trailing newline is still returned.
- There is no line length limit: a record is buffered until its terminator arrives, so a file that is truncated
  mid-record is held in memory as one very long line.
- Errors carry the physical line number: `Error::Json { line, .. }`.
- After a decode error the stream ends — that is stock `FramedRead` behavior, so a file with one bad line cannot be read
  past it in place.

## Defaults

| Setting                            | Default      | Configure with                      |
|------------------------------------|--------------|-------------------------------------|
| Buffered bytes before a write      | 64 KiB       | `JsonlWriter::with_flush_threshold` |
| Flush interval (background writer) | 1 s          | `WriterConfig::flush_interval`      |
| Queue depth before writers wait    | 1024 records | `WriterConfig::queue_capacity`      |
| Read buffer                        | 64 KiB       | `JsonlReader::new` / `FramedRead`   |

## Performance

Medians from `cargo bench` (`benches/throughput.rs`) on an Apple M3 Pro, release profile, 2,000 records of ~120 bytes
per iteration. The file rows append to a temp file and issue no `fsync`, so they measure the page cache: trust the
ratios, not the absolute numbers.

| Benchmark               | Time / 2,000 records | Throughput   | What it measures                              |
|-------------------------|----------------------|--------------|-----------------------------------------------|
| `encode`                | 175 µs               | 11.4 M rec/s | serialization alone, no I/O                   |
| `append_buffered`       | 322 µs               | 6.2 M rec/s  | the normal path: batch, then flush            |
| `append_via_handle`     | 506 µs               | 3.9 M rec/s  | +~90 ns per record for the `spawn_writer` hop |
| `append_flush_each`     | 29.3 ms              | 68 K rec/s   | flushing every record: **91× slower**         |
| `decode`                | 474 µs               | 4.2 M rec/s  | line framing plus `serde_json`                |
| `decode_serde_baseline` | 336 µs               | 5.9 M rec/s  | the same bytes straight through `serde_json`  |

The two numbers that matter: batching beats per-record flushing by two orders of magnitude, and line framing costs about
40% on top of raw `serde_json` when decoding.

## Using the codec directly

`JsonlCodec` is the only piece that knows the format; everything else is sugar. It works anywhere a `Decoder` or
`Encoder` fits:

```rust
use bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder};

let mut codec = tokio_jsonl::JsonlCodec::<serde_json::Value>::new();
let mut buf = BytesMut::new();
codec.encode(serde_json::json!({ "level": "info" }), &mut buf)?;
assert_eq!(&buf[..], b"{\"level\":\"info\"}\n");

while let Some(value) = codec.decode(&mut buf)? {
    println!("{value}");
}
```

Pass it to `FramedRead` / `FramedWrite` to get `Stream` / `Sink` access — that needs `futures-util` (for `StreamExt` and
`SinkExt`) or `tokio-stream` (for `StreamExt`) in your own dependencies. Note that `SinkExt::send` flushes after every
record; use `feed` plus a periodic `flush` if you are appending in bulk.
