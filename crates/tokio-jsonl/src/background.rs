//! Handing records to a background task that owns the file.

use std::path::Path;
use std::time::Duration;

use serde::Serialize;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{self, MissedTickBehavior};

use crate::codec::Error;
use crate::writer::{DEFAULT_FLUSH_THRESHOLD, JsonlWriter};

/// Tuning for [`spawn_writer`].
#[derive(Debug, Clone)]
pub struct WriterConfig {
    /// How often the background task flushes buffered records, even when
    /// nothing new arrived. Defaults to 1 second.
    pub flush_interval: Duration,
    /// Buffered bytes that trigger a write out. Defaults to
    /// [`DEFAULT_FLUSH_THRESHOLD`].
    pub flush_threshold: usize,
    /// Number of records that may be queued before [`JsonlHandle::write`]
    /// starts waiting. Defaults to 1024.
    pub queue_capacity: usize,
}

impl Default for WriterConfig {
    fn default() -> Self {
        Self {
            flush_interval: Duration::from_secs(1),
            flush_threshold: DEFAULT_FLUSH_THRESHOLD,
            queue_capacity: 1024,
        }
    }
}

impl WriterConfig {
    /// Starts from the defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets how often buffered records are flushed.
    pub fn flush_interval(mut self, interval: Duration) -> Self {
        self.flush_interval = interval;
        self
    }

    /// Sets how many buffered bytes trigger a write out.
    pub fn flush_threshold(mut self, bytes: usize) -> Self {
        self.flush_threshold = bytes;
        self
    }

    /// Sets how many records may be queued before writers wait.
    pub fn queue_capacity(mut self, records: usize) -> Self {
        self.queue_capacity = records.max(1);
        self
    }
}

enum Command<T> {
    Write(T),
    Flush(oneshot::Sender<Result<(), Error>>),
}

/// Cheap, cloneable handle to a background appender.
///
/// Dropping every handle shuts the background task down; the task flushes
/// whatever it still holds before exiting, but there is no way to await that,
/// so call [`flush`](Self::flush) first if it matters.
pub struct JsonlHandle<T> {
    tx: mpsc::Sender<Command<T>>,
}

impl<T> Clone for JsonlHandle<T> {
    fn clone(&self) -> Self {
        Self { tx: self.tx.clone() }
    }
}

impl<T: Serialize> JsonlHandle<T> {
    /// Queues one record, waiting if the queue is full.
    ///
    /// Returns [`Error::Io`] with `BrokenPipe` once the background task stopped,
    /// which happens if a previous write failed.
    pub async fn write(&self, record: T) -> Result<(), Error> {
        self.tx.send(Command::Write(record)).await.map_err(|_| task_gone())
    }

    /// Waits until everything queued so far has been written and flushed.
    pub async fn flush(&self) -> Result<(), Error> {
        let (ack, done) = oneshot::channel();
        self.tx.send(Command::Flush(ack)).await.map_err(|_| task_gone())?;
        done.await.map_err(|_| task_gone())?
    }
}

impl<T> std::fmt::Debug for JsonlHandle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JsonlHandle").field("closed", &self.tx.is_closed()).finish_non_exhaustive()
    }
}

fn task_gone() -> Error {
    Error::Io(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "background writer has stopped"))
}

/// Appends `T` records to `path` from a background task, with the default
/// [`WriterConfig`].
pub async fn spawn_writer<T>(path: impl AsRef<Path>) -> std::io::Result<JsonlHandle<T>>
where
    T: Serialize + Send + 'static,
{
    spawn_writer_with(path, WriterConfig::default()).await
}

/// Appends `T` records to `path` from a background task.
///
/// The task owns the file, flushes on `config.flush_interval`, and flushes once
/// more when the last handle is dropped. Write errors are not reported to the
/// caller: they stop the task, and later [`JsonlHandle::write`] calls fail with
/// `BrokenPipe`.
pub async fn spawn_writer_with<T>(
    path: impl AsRef<Path>,
    config: WriterConfig,
) -> std::io::Result<JsonlHandle<T>>
where
    T: Serialize + Send + 'static,
{
    let writer = JsonlWriter::open(path).await?.with_flush_threshold(config.flush_threshold);
    let (tx, mut rx) = mpsc::channel::<Command<T>>(config.queue_capacity);

    tokio::spawn(async move {
        let mut ticker = time::interval(config.flush_interval);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        ticker.tick().await; // the first tick completes immediately

        let mut writer = writer;
        loop {
            tokio::select! {
                command = rx.recv() => match command {
                    Some(Command::Write(record)) => {
                        // Encode without holding `&record` across an await, so
                        // the task only needs `T: Send`, not `T: Sync`.
                        let encoded = writer.encode(&record);
                        if encoded.is_err() || writer.spill_if_full().await.is_err() {
                            break;
                        }
                    }
                    Some(Command::Flush(ack)) => {
                        let result = writer.flush().await;
                        let failed = result.is_err();
                        let _ = ack.send(result);
                        if failed {
                            break;
                        }
                    }
                    None => break,
                },
                _ = ticker.tick() => {
                    if writer.flush().await.is_err() {
                        break;
                    }
                }
            }
        }
        let _ = writer.close().await;
    });

    Ok(JsonlHandle { tx })
}
