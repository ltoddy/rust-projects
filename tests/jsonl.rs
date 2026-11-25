use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio_jsonl::{Error, JsonlReader, JsonlWriter, WriterConfig};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Rec {
    id: u64,
    name: String,
}

fn rec(id: u64, name: &str) -> Rec {
    Rec {
        id,
        name: name.to_owned(),
    }
}

async fn collect(input: &[u8]) -> Vec<Result<Rec, Error>> {
    let mut reader = JsonlReader::<_, Rec>::new(input);
    let mut out = Vec::new();
    while let Some(item) = reader.next().await {
        out.push(item);
    }
    out
}

#[tokio::test]
async fn decodes_blank_lines_crlf_and_unterminated_tail() {
    let input: &[u8] = b"\n\r\n{\"id\":1,\"name\":\"a\"}\r\n{\"id\":2,\"name\":\"b\"}\n  \n{\"id\":3,\"name\":\"c\"}";

    let got: Vec<Rec> = collect(input)
        .await
        .into_iter()
        .map(Result::unwrap)
        .collect();

    assert_eq!(got, vec![rec(1, "a"), rec(2, "b"), rec(3, "c")]);
}

#[tokio::test]
async fn reports_the_physical_line_number() {
    let input: &[u8] = b"{\"id\":1,\"name\":\"a\"}\n\nnot json\n{\"id\":2,\"name\":\"b\"}\n";
    let mut reader = JsonlReader::<_, Rec>::new(input);

    reader.next().await.unwrap().unwrap();

    match reader.next().await.unwrap() {
        Err(Error::Json { line, .. }) => assert_eq!(line, 3),
        other => panic!("expected a JSON error, got {other:?}"),
    }
}

#[tokio::test]
async fn a_malformed_line_ends_the_stream_with_its_line_number() {
    let input: &[u8] = b"{\"id\":1,\"name\":\"a\"}\nnot json\n{\"id\":2,\"name\":\"b\"}\n";
    let mut reader = JsonlReader::<_, Rec>::new(input);

    assert_eq!(reader.next().await.unwrap().unwrap(), rec(1, "a"));
    match reader.next().await.unwrap() {
        Err(Error::Json { line, .. }) => assert_eq!(line, 2),
        other => panic!("expected a JSON error, got {other:?}"),
    }
    assert!(reader.next().await.is_none());
}

#[tokio::test]
async fn decodes_lines_of_any_length() {
    let long = "a".repeat(4 * 1024 * 1024);
    let input = format!("{}\n", serde_json::json!({ "id": 1, "name": long }));
    let mut reader = JsonlReader::<_, Rec>::new(input.as_bytes());

    assert_eq!(reader.next().await.unwrap().unwrap().name.len(), long.len());
    assert!(reader.next().await.is_none());
}

#[tokio::test]
async fn write_all_truncates_and_read_all_roundtrips() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("one_shot.jsonl");

    tokio_jsonl::write_all(&path, vec![rec(1, "a"), rec(2, "b")])
        .await
        .unwrap();
    tokio_jsonl::write_all(&path, vec![rec(3, "c")])
        .await
        .unwrap();

    let got: Vec<Rec> = tokio_jsonl::read_all(&path).await.unwrap();
    assert_eq!(got, vec![rec(3, "c")]);
}

#[tokio::test]
async fn append_all_keeps_previous_records() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("one_shot_append.jsonl");

    tokio_jsonl::write_all(&path, vec![rec(1, "a")])
        .await
        .unwrap();
    tokio_jsonl::append_all(&path, vec![rec(2, "b"), rec(3, "c")])
        .await
        .unwrap();

    let got: Vec<Rec> = tokio_jsonl::read_all(&path).await.unwrap();
    assert_eq!(got, vec![rec(1, "a"), rec(2, "b"), rec(3, "c")]);

    let content = tokio::fs::read_to_string(&path).await.unwrap();
    assert!(
        content.ends_with('\n'),
        "the file must not be left unterminated"
    );
}

#[tokio::test]
async fn read_all_fails_on_the_first_bad_line() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("broken.jsonl");
    tokio::fs::write(
        &path,
        b"{\"id\":1,\"name\":\"a\"}\nnot json\n{\"id\":2,\"name\":\"b\"}\n",
    )
    .await
    .unwrap();

    match tokio_jsonl::read_all::<Rec>(&path).await {
        Err(Error::Json { line, .. }) => assert_eq!(line, 2),
        Err(other) => panic!("expected a JSON error, got {other:?}"),
        Ok(records) => panic!("expected an error, decoded {} records", records.len()),
    }
}

#[tokio::test]
async fn read_all_of_an_empty_file_is_empty() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty.jsonl");
    tokio::fs::write(&path, b"").await.unwrap();

    let got: Vec<Rec> = tokio_jsonl::read_all(&path).await.unwrap();
    assert!(got.is_empty());

    match tokio_jsonl::read_all::<Rec>(dir.path().join("missing.jsonl")).await {
        Err(Error::Io(err)) => assert_eq!(err.kind(), std::io::ErrorKind::NotFound),
        other => panic!("expected an io error, got {other:?}"),
    }
}

#[tokio::test]
async fn appending_across_runs_keeps_previous_records() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("data.jsonl");

    let mut writer = JsonlWriter::<_, Rec>::create(&path).await.unwrap();
    writer.write(&rec(1, "a")).await.unwrap();
    writer.close().await.unwrap();

    let mut writer = JsonlWriter::<_, Rec>::open(&path).await.unwrap();
    writer.write(&rec(2, "b")).await.unwrap();
    writer.write(&rec(3, "c")).await.unwrap();
    writer.close().await.unwrap();

    let content = tokio::fs::read_to_string(&path).await.unwrap();
    assert_eq!(content.lines().count(), 3);
    assert!(
        content.ends_with('\n'),
        "every record must end with a newline"
    );

    let got: Vec<Rec> = collect(content.as_bytes())
        .await
        .into_iter()
        .map(Result::unwrap)
        .collect();
    assert_eq!(got, vec![rec(1, "a"), rec(2, "b"), rec(3, "c")]);
}

#[tokio::test]
async fn records_stay_in_memory_until_the_threshold_is_reached() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("buffered.jsonl");

    let mut writer = JsonlWriter::<_, Rec>::open(&path)
        .await
        .unwrap()
        .with_flush_threshold(64 * 1024);
    writer.write(&rec(1, "a")).await.unwrap();
    assert_eq!(tokio::fs::metadata(&path).await.unwrap().len(), 0);
    assert!(writer.buffered_bytes() > 0);

    writer.flush().await.unwrap();
    assert!(tokio::fs::metadata(&path).await.unwrap().len() > 0);
    writer.close().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn background_writer_handles_concurrent_producers() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("metrics.jsonl");
    let config = WriterConfig::new()
        .flush_interval(Duration::from_millis(20))
        .flush_threshold(4096)
        .queue_capacity(16);

    let handle = tokio_jsonl::spawn_writer_with::<Rec>(&path, config)
        .await
        .unwrap();

    let mut workers = Vec::new();
    for task in 0..8u64 {
        let handle = handle.clone();
        workers.push(tokio::spawn(async move {
            for i in 0..50u64 {
                handle.write(rec(task * 50 + i, "x")).await.unwrap();
            }
        }));
    }
    for worker in workers {
        worker.await.unwrap();
    }
    handle.flush().await.unwrap();
    drop(handle);

    let mut reader = JsonlReader::<_, Rec>::open(&path).await.unwrap();
    let mut ids = Vec::new();
    while let Some(item) = reader.next().await {
        ids.push(item.unwrap().id);
    }
    ids.sort_unstable();
    assert_eq!(ids, (0..400).collect::<Vec<_>>());
}
