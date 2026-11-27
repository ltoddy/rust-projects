//! A [`tokio_util::codec`] implementation for [JSON Lines](https://jsonlines.org/).

use std::marker::PhantomData;

use bytes::{BufMut, BytesMut};
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio_util::codec::{Decoder, Encoder};

/// Errors produced while framing (and serializing) JSON Lines.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An I/O failure of the underlying reader or writer.
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),

    /// A line that does not parse into the expected type.
    #[error("line {line} is not valid JSON: {source}")]
    Json {
        /// 1-based number of the failing line.
        line: u64,
        /// The underlying `serde_json` error.
        source: serde_json::Error,
    },

    /// A value that could not be serialized.
    #[error("failed to serialize record: {0}")]
    Serialize(serde_json::Error),
}

/// Encodes `T` as one JSON value per line and decodes lines back into `T`.
///
/// When decoding, blank lines are skipped and a trailing `\r` (from `\r\n`
/// terminators) is stripped, so files produced on Windows parse the same way as
/// files produced on unix. A final line without a terminator is still emitted.
///
/// The decoder keeps a line counter, which is reported in [`Error::Json`] so a
/// broken record can be located in a large file.
pub struct JsonlCodec<T> {
    line: u64,
    _item: PhantomData<fn() -> T>,
}

impl<T> Default for JsonlCodec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> std::fmt::Debug for JsonlCodec<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JsonlCodec").field("line", &self.line).finish_non_exhaustive()
    }
}

impl<T> JsonlCodec<T> {
    /// Creates a codec positioned at the first line.
    pub fn new() -> Self {
        Self { line: 0, _item: PhantomData }
    }

    /// Number of lines consumed so far (1-based, after the first record).
    pub fn lines_read(&self) -> u64 {
        self.line
    }

    /// Splits the next `\n`-terminated line off `src`, stripping the terminator.
    fn take_line(&mut self, src: &mut BytesMut) -> Option<BytesMut> {
        let pos = src.iter().position(|b| *b == b'\n')?;
        let mut line = src.split_to(pos + 1);
        line.truncate(pos);
        if line.last() == Some(&b'\r') {
            line.truncate(line.len() - 1);
        }
        self.line += 1;
        Some(line)
    }

    fn parse(&self, line: &[u8]) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        serde_json::from_slice(line).map_err(|source| Error::Json { line: self.line, source })
    }
}

impl<T: DeserializeOwned> Decoder for JsonlCodec<T> {
    type Item = T;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        loop {
            let Some(line) = self.take_line(src) else {
                return Ok(None);
            };

            if line.iter().all(u8::is_ascii_whitespace) {
                continue;
            }
            return self.parse(&line).map(Some);
        }
    }

    fn decode_eof(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if let Some(item) = self.decode(src)? {
            return Ok(Some(item));
        }
        if src.is_empty() {
            return Ok(None);
        }

        // Unterminated last line: jsonl in the wild often lacks the final `\n`.
        let mut line = src.split_to(src.len());
        self.line += 1;
        if line.last() == Some(&b'\r') {
            line.truncate(line.len() - 1);
        }
        if line.iter().all(u8::is_ascii_whitespace) {
            return Ok(None);
        }
        self.parse(&line).map(Some)
    }
}

impl<T: Serialize> Encoder<T> for JsonlCodec<T> {
    type Error = Error;

    fn encode(&mut self, item: T, dst: &mut BytesMut) -> Result<(), Self::Error> {
        encode_record(&item, dst)
    }
}

impl<T: Serialize> Encoder<&T> for JsonlCodec<T> {
    type Error = Error;

    fn encode(&mut self, item: &T, dst: &mut BytesMut) -> Result<(), Self::Error> {
        encode_record(item, dst)
    }
}

/// Serializes `item` and appends it plus a `\n` to `dst`.
fn encode_record<T: Serialize + ?Sized>(item: &T, dst: &mut BytesMut) -> Result<(), Error> {
    let start = dst.len();
    let result = {
        let mut writer = (&mut *dst).writer();
        serde_json::to_writer(&mut writer, item)
    };
    if let Err(source) = result {
        // Do not leave a half written record in the buffer.
        dst.truncate(start);
        return Err(Error::Serialize(source));
    }
    dst.put_u8(b'\n');
    Ok(())
}
