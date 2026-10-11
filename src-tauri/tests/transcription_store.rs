use bilicatch_lib::{
    infrastructure::transcription::json_store::JsonTranscriptionStore,
    services::transcription::TranscriptionStore,
};

#[test]
fn corrupt_store_is_reported_without_overwriting_the_file() {
    let root = tempfile::tempdir().expect("store directory");
    let path = root.path().join("transcriptions.json");
    std::fs::write(&path, b"not json").expect("corrupt fixture");
    let store = JsonTranscriptionStore::new(&path);

    let error = store.load().expect_err("corrupt store must fail");

    assert_eq!(
        error.details.as_deref(),
        Some("TRANSCRIPTION_STORE_CORRUPT")
    );
    assert_eq!(std::fs::read(&path).unwrap(), b"not json");
}
