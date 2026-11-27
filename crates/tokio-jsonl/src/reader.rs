//! Reading JSON Lines from an [`AsyncRead`].

use serde::de::DeserializeOwned;
use tokio::io::AsyncRead;
use tokio_stream::StreamExt;
use tokio_util::codec::FramedRead;

use crate::codec::{Error, JsonlCodec};

/// Read buffer size used by the file constructors (64 KiB).
pub const DEFAULT_READ_CAPACITY: usize = 64 * 1024;

/// Reads `T` values out of an [`AsyncRead`].
pub struct JsonlReader<R, T> {
    inner: FramedRead<R, JsonlCodec<T>>,
}

impl<R, T> std::fmt::Debug for JsonlReader<R, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JsonlReader").finish_non_exhaustive()
    }
}

impl<R: AsyncRead + Unpin, T: DeserializeOwned> JsonlReader<R, T> {
    /// Wraps `reader`, decoding one JSON value per line.
    pub fn new(reader: R) -> Self {
        Self { inner: FramedRead::with_capacity(reader, JsonlCodec::new(), DEFAULT_READ_CAPACITY) }
    }

    /// Returns the next record, or `None` at the end of input.
    ///
    /// A malformed line is reported once as [`Error::Json`] with its line
    /// number. Like every [`FramedRead`], the stream ends after a decode error,
    /// so reading a file that may contain bad lines means handling the error and
    /// carrying on with a fresh reader positioned past the bad line.
    pub async fn next(&mut self) -> Option<Result<T, Error>> {
        StreamExt::next(&mut self.inner).await
    }
}

impl<R, T> JsonlReader<R, T> {
    /// Access to the underlying [`FramedRead`], e.g. to use it as a `Stream`.
    pub fn inner(&self) -> &FramedRead<R, JsonlCodec<T>> {
        &self.inner
    }

    /// Unwraps the [`FramedRead`].
    pub fn into_inner(self) -> FramedRead<R, JsonlCodec<T>> {
        self.inner
    }
}

#[cfg(feature = "fs")]
impl<T: DeserializeOwned> JsonlReader<tokio::fs::File, T> {
    /// Opens `path` for buffered reading.
    pub async fn open(path: impl AsRef<std::path::Path>) -> std::io::Result<Self> {
        let file = tokio::fs::File::open(path).await?;
        Ok(Self::new(file))
    }
}
