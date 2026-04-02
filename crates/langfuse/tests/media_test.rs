use langfuse::media::types::{LangfuseMedia, parse_media_references};

#[test]
fn test_media_from_data_uri() {
    let uri = "data:image/png;base64,iVBORw0KGgo=";
    let media = LangfuseMedia::from_data_uri(uri).unwrap();
    assert_eq!(media.content_type, "image/png");
    assert!(!media.data.is_empty());
}

#[test]
fn test_media_from_data_uri_invalid() {
    let result = LangfuseMedia::from_data_uri("invalid");
    assert!(result.is_err());
}

#[test]
fn test_media_from_bytes() {
    let media = LangfuseMedia::from_bytes("text/plain", b"hello".to_vec());
    assert_eq!(media.content_type, "text/plain");
    assert_eq!(media.data, b"hello");
}

#[test]
fn test_media_size() {
    let media = LangfuseMedia::from_bytes("text/plain", vec![1, 2, 3]);
    assert_eq!(media.size(), 3);
}

#[test]
fn test_parse_media_references() {
    let text =
        "Here is an image: @@@langfuseMedia:type=image/png|id=abc123|source=base64_data_uri@@@ end";
    let refs = parse_media_references(text);
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].content_type, "image/png");
    assert_eq!(refs[0].media_id, "abc123");
    assert_eq!(refs[0].source, "base64_data_uri");
}

#[test]
fn test_parse_multiple_media_references() {
    let text = "@@@langfuseMedia:type=image/png|id=a|source=s1@@@ text @@@langfuseMedia:type=audio/mp3|id=b|source=s2@@@";
    let refs = parse_media_references(text);
    assert_eq!(refs.len(), 2);
}

#[test]
fn test_parse_no_media_references() {
    let refs = parse_media_references("no media here");
    assert!(refs.is_empty());
}

// ---------------------------------------------------------------------------
// Async file reading
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_media_from_file_async() {
    // Write a temp file, read it back via from_file_async.
    let dir = std::env::temp_dir().join("langfuse_test_media");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("test.txt");
    std::fs::write(&path, b"async hello").unwrap();

    let media = LangfuseMedia::from_file_async("text/plain", &path)
        .await
        .unwrap();
    assert_eq!(media.content_type, "text/plain");
    assert_eq!(media.data, b"async hello");

    // Cleanup
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn test_media_from_file_async_not_found() {
    let result = LangfuseMedia::from_file_async("text/plain", "/nonexistent/file.txt").await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("File read error"), "unexpected error: {err}");
}

// ---------------------------------------------------------------------------
// MediaManager construction
// ---------------------------------------------------------------------------

#[test]
fn test_media_manager_is_cloneable() {
    use langfuse::media::manager::MediaManager;
    let config = langfuse::LangfuseConfig::builder()
        .public_key("pk-test")
        .secret_key("sk-test")
        .build()
        .unwrap();
    let manager = MediaManager::new(&config);
    let _cloned = manager.clone();
}
