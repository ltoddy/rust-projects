//! One-shot helpers that mirror [`std::fs`] / [`tokio::fs`].

use std::path::Path;

use serde::{Serialize, de::DeserializeOwned};
use tokio::io::AsyncWrite;

use crate::codec::Error;
use crate::reader::JsonlReader;
use crate::writer::JsonlWriter;

/// Reads every record from `path` into memory.
///
/// This is the `fs::read_to_string` of this crate: the whole file is decoded
/// into a `Vec`, and a single malformed line fails the call with
/// [`Error::Json`] carrying that line's number.
///
/// Use [`JsonlReader`] for files that may not fit in memory, or to read a file
/// that is still being appended to.
pub async fn read_all<T>(path: impl AsRef<Path>) -> Result<Vec<T>, Error>
where
    T: DeserializeOwned,
{
    let mut reader = JsonlReader::<_, T>::open(path).await?;
    let mut records = Vec::new();
    while let Some(record) = reader.next().await {
        records.push(record?);
    }
    Ok(records)
}

/// Truncates `path` (creating it if missing) and writes `records` to it.
///
/// Records that were already in the file are discarded; use [`append_all`] to
/// keep them.
pub async fn write_all<T>(
    path: impl AsRef<Path>,
    records: impl IntoIterator<Item = T>,
) -> Result<(), Error>
where
    T: Serialize,
{
    let writer = JsonlWriter::<_, T>::create(path).await?;
    write_records(writer, records).await
}

/// Appends `records` to `path`, creating it if missing.
///
/// The file is opened in append mode, so existing records are kept.
pub async fn append_all<T>(
    path: impl AsRef<Path>,
    records: impl IntoIterator<Item = T>,
) -> Result<(), Error>
where
    T: Serialize,
{
    let writer = JsonlWriter::<_, T>::open(path).await?;
    write_records(writer, records).await
}

async fn write_records<W, T>(
    mut writer: JsonlWriter<W, T>,
    records: impl IntoIterator<Item = T>,
) -> Result<(), Error>
where
    W: AsyncWrite + Unpin,
    T: Serialize,
{
    for record in records {
        if let Err(err) = writer.encode(&record) {
            // Keep the records that were already accepted, then report.
            writer.flush().await?;
            return Err(err);
        }
        writer.spill_if_full().await?;
    }
    writer.close().await
}
