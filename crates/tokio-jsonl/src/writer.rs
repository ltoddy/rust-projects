//! Buffered, append-oriented writing of JSON Lines.

use std::path::Path;

use bytes::BytesMut;
use serde::Serialize;
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tokio_util::codec::Encoder;

use crate::codec::{Error, JsonlCodec};

/// How many encoded bytes are buffered before they are handed to the inner
/// writer (default 64 KiB).
pub const DEFAULT_FLUSH_THRESHOLD: usize = 64 * 1024;

/// Writes `T` values as JSON Lines to an [`AsyncWrite`].
///
/// Records are serialized into an in-memory buffer and only written out when
/// the buffer reaches the flush threshold, or when [`flush`](Self::flush) /
/// [`close`](Self::close) is called. This keeps `write` from doing a syscall
/// per record, which is what matters when appending to a file continuously.
///
/// Because of that buffering, **data buffered but not yet flushed is lost if
/// the writer is dropped**. Call [`flush`](Self::flush) or
/// [`close`](Self::close) when shutting down, or use [`crate::spawn_writer`],
/// which flushes on an interval and on exit.
pub struct JsonlWriter<W, T> {
    writer: W,
    codec: JsonlCodec<T>,
    buf: BytesMut,
    flush_threshold: usize,
}

impl<W, T> std::fmt::Debug for JsonlWriter<W, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JsonlWriter")
            .field("buffered_bytes", &self.buf.len())
            .field("flush_threshold", &self.flush_threshold)
            .finish_non_exhaustive()
    }
}

impl<W: AsyncWrite + Unpin, T: Serialize> JsonlWriter<W, T> {
    /// Wraps `writer`, buffering up to [`DEFAULT_FLUSH_THRESHOLD`] bytes.
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            codec: JsonlCodec::new(),
            buf: BytesMut::with_capacity(DEFAULT_FLUSH_THRESHOLD),
            flush_threshold: DEFAULT_FLUSH_THRESHOLD,
        }
    }

    /// Sets how many buffered bytes trigger an automatic write out.
    ///
    /// A threshold of `1` effectively makes every record hit the inner writer,
    /// while still coalescing nothing.
    pub fn with_flush_threshold(mut self, bytes: usize) -> Self {
        self.flush_threshold = bytes.max(1);
        self
    }

    /// Serializes one record into the buffer. Does no I/O.
    pub fn encode(&mut self, record: &T) -> Result<(), Error> {
        self.codec.encode(record, &mut self.buf)
    }

    /// Writes the buffer out once it reached the flush threshold.
    ///
    /// Split from [`encode`](Self::encode) so a caller can serialize without
    /// holding a borrow across an `.await`, which keeps `write` futures `Send`
    /// for records that are `Send` but not `Sync`.
    pub async fn spill_if_full(&mut self) -> Result<(), Error> {
        if self.buf.len() >= self.flush_threshold {
            self.spill().await?;
        }
        Ok(())
    }

    /// Serializes one record into the buffer, writing out the buffer once it
    /// has grown past the flush threshold.
    pub async fn write(&mut self, record: &T) -> Result<(), Error> {
        self.encode(record)?;
        self.spill_if_full().await
    }

    /// Writes buffered records to the inner writer without flushing it.
    async fn spill(&mut self) -> Result<(), Error> {
        if self.buf.is_empty() {
            return Ok(());
        }
        self.writer.write_all(&self.buf).await?;
        self.buf.clear();
        Ok(())
    }

    /// Writes out buffered records and flushes the inner writer.
    pub async fn flush(&mut self) -> Result<(), Error> {
        self.spill().await?;
        self.writer.flush().await?;
        Ok(())
    }

    /// Flushes and shuts the inner writer down.
    pub async fn close(mut self) -> Result<(), Error> {
        self.flush().await?;
        self.writer.shutdown().await?;
        Ok(())
    }

    /// Encoded bytes waiting to be written out.
    pub fn buffered_bytes(&self) -> usize {
        self.buf.len()
    }

    /// The wrapped writer.
    pub fn get_ref(&self) -> &W {
        &self.writer
    }

    /// The wrapped writer, mutably.
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.writer
    }

    /// Unwraps the inner writer, discarding buffered records.
    pub fn into_inner(self) -> W {
        self.writer
    }
}

impl<T: Serialize> JsonlWriter<tokio::fs::File, T> {
    /// Creates (or truncates) `path` and writes a new jsonl file.
    pub async fn create(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let file = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)
            .await?;
        Ok(Self::new(file))
    }

    /// Opens `path` for appending, creating it if it does not exist.
    ///
    /// The file is opened with `O_APPEND`, so each write lands at the end of
    /// the file without an explicit seek. As long as every write covers whole
    /// records (which is the case here), appending from several processes does
    /// not interleave *within* a write, though ordering between them is
    /// arbitrary. For guaranteed durability of every record, use one writer per
    /// file and call [`sync`](Self::sync).
    pub async fn open(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await?;
        Ok(Self::new(file))
    }

    /// Flushes buffered records and asks the OS to persist them (`fsync`).
    pub async fn sync(&mut self) -> Result<(), Error> {
        self.flush().await?;
        self.writer.sync_all().await?;
        Ok(())
    }
}
