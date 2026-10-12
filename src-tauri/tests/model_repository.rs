use std::sync::Arc;

use bilicatch_lib::{
    infrastructure::transcription::model_repository::{
        official_model_manifest, FileModelRepository, ModelDownloadCancellation, ModelManifestEntry,
    },
    models::{AppErrorCode, AsrModelId},
};
use tempfile::TempDir;

fn fixture_manifest() -> Vec<ModelManifestEntry> {
    vec![ModelManifestEntry {
        id: AsrModelId::Tiny,
        display_name: "Tiny".into(),
        file_name: "ggml-tiny.bin".into(),
        size_bytes: 3,
        sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into(),
        official_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin"
            .into(),
        recommended_for: "test".into(),
        capabilities: vec!["multilingual".into()],
    }]
}

fn repository(root: &TempDir) -> FileModelRepository {
    FileModelRepository::with_manifest(root.path(), fixture_manifest())
}

#[test]
fn official_manifest_pins_all_approved_models() {
    let entries = official_model_manifest();

    assert_eq!(
        entries.iter().map(|entry| entry.id).collect::<Vec<_>>(),
        vec![
            AsrModelId::Tiny,
            AsrModelId::Base,
            AsrModelId::Small,
            AsrModelId::Medium,
            AsrModelId::LargeV3,
            AsrModelId::FireRedAsr2,
        ]
    );
    assert_eq!(entries[0].size_bytes, 77_691_713);
    assert_eq!(
        entries[0].sha256,
        "be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21"
    );
    assert_eq!(entries[4].size_bytes, 3_095_033_483);
    assert_eq!(
        entries[4].sha256,
        "64d182b440b98d5203c4f9bd541544d84c605196c4f7b845dfa11fb23594d1e2"
    );
    assert_eq!(entries[5].size_bytes, 775_861_420);
    assert_eq!(
        entries[5].sha256,
        "ca3dbabd82170110cc0b343c2890866d449984bc9cd92b9a18371ff80a81bb99"
    );
}

#[test]
fn scan_marks_only_size_and_sha256_matching_files_as_valid() {
    let root = tempfile::tempdir().expect("model directory");
    std::fs::write(root.path().join("ggml-tiny.bin"), b"abc").expect("fixture model");
    let models = repository(&root)
        .list(AsrModelId::Tiny)
        .expect("model scan should succeed");

    assert_eq!(models.len(), 1);
    assert!(models[0].installed);
    assert!(models[0].valid);
    assert!(models[0].is_default);

    std::fs::write(root.path().join("ggml-tiny.bin"), b"abd").expect("corrupt model");
    let models = repository(&root)
        .list(AsrModelId::Tiny)
        .expect("model scan should succeed");
    assert!(models[0].installed);
    assert!(!models[0].valid);
}

#[test]
fn publishing_a_part_file_verifies_then_renames_atomically() {
    let root = tempfile::tempdir().expect("model directory");
    let repo = repository(&root);
    let part = root.path().join("ggml-tiny.bin.part");
    std::fs::write(&part, b"abc").expect("part file");

    let model = repo
        .publish_part(AsrModelId::Tiny, &part, AsrModelId::Tiny)
        .expect("valid part should publish");

    assert!(model.valid);
    assert!(!part.exists());
    assert_eq!(
        std::fs::read(root.path().join("ggml-tiny.bin")).unwrap(),
        b"abc"
    );
}

#[test]
fn corrupt_parts_and_final_files_are_removed_by_explicit_verification() {
    let root = tempfile::tempdir().expect("model directory");
    let repo = repository(&root);
    let final_path = root.path().join("ggml-tiny.bin");
    let part_path = root.path().join("ggml-tiny.bin.part");
    std::fs::write(&final_path, b"abd").expect("corrupt final");
    std::fs::write(&part_path, b"partial").expect("partial download");

    let error = repo
        .verify(AsrModelId::Tiny, AsrModelId::Tiny)
        .expect_err("corrupt model must fail verification");

    assert_eq!(error.code, AppErrorCode::E017);
    assert!(!final_path.exists());
    assert!(!part_path.exists());
}

#[test]
fn deleting_an_in_use_model_is_rejected_without_touching_the_file() {
    let root = tempfile::tempdir().expect("model directory");
    let repo = repository(&root);
    let path = root.path().join("ggml-tiny.bin");
    std::fs::write(&path, b"abc").expect("fixture model");

    let error = repo
        .delete(AsrModelId::Tiny, true)
        .expect_err("in-use model must not be deleted");

    assert_eq!(error.code, AppErrorCode::E017);
    assert_eq!(error.details.as_deref(), Some("MODEL_IN_USE"));
    assert!(path.exists());
}

#[test]
fn custom_sources_replace_only_the_https_base_url() {
    let root = tempfile::tempdir().expect("model directory");
    let repo = Arc::new(repository(&root));

    assert_eq!(
        repo.download_url(AsrModelId::Tiny, "https://mirror.example/models/")
            .expect("HTTPS mirror should resolve")
            .as_str(),
        "https://mirror.example/models/ggml-tiny.bin"
    );
    let error = repo
        .download_url(AsrModelId::Tiny, "http://mirror.example/models")
        .expect_err("HTTP mirror must be rejected");
    assert_eq!(error.code, AppErrorCode::E012);
}

#[test]
fn download_writer_streams_to_part_then_publishes_after_verification() {
    let root = tempfile::tempdir().expect("model directory");
    let repo = repository(&root);
    let cancellation = ModelDownloadCancellation::new();
    let mut writer = repo
        .begin_download(AsrModelId::Tiny, AsrModelId::Tiny, cancellation)
        .expect("download writer");

    assert_eq!(writer.append(b"a").expect("first chunk"), 1);
    assert_eq!(writer.append(b"bc").expect("second chunk"), 3);
    let model = writer.finish().expect("verified download should publish");

    assert!(model.valid);
    assert!(!root.path().join("ggml-tiny.bin.part").exists());
    assert_eq!(
        std::fs::read(root.path().join("ggml-tiny.bin")).unwrap(),
        b"abc"
    );
}

#[test]
fn cancelling_a_stream_removes_the_partial_file() {
    let root = tempfile::tempdir().expect("model directory");
    let repo = repository(&root);
    let cancellation = ModelDownloadCancellation::new();
    let mut writer = repo
        .begin_download(AsrModelId::Tiny, AsrModelId::Tiny, cancellation.clone())
        .expect("download writer");
    writer.append(b"a").expect("first chunk");

    cancellation.cancel();
    let error = writer
        .append(b"bc")
        .expect_err("cancelled stream must stop");
    drop(writer);

    assert_eq!(error.code, AppErrorCode::E012);
    assert_eq!(error.details.as_deref(), Some("MODEL_DOWNLOAD_CANCELLED"));
    assert!(!root.path().join("ggml-tiny.bin.part").exists());
    assert!(!root.path().join("ggml-tiny.bin").exists());
}
