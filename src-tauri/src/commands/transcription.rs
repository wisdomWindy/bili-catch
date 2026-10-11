use std::sync::Arc;

use tauri::State;

use crate::{
    infrastructure::transcription::runtime::TranscriptionRuntime,
    models::{
        AppError, AsrModel, AsrModelListSnapshot, ControlTranscribeTaskRequest,
        CreateTranscribeTaskRequest, CreateTranscribeTaskResult, ExportTranscriptRequest,
        ExportTranscriptResult, InspectTranscriptionSourceRequest, MediaSourceInfo, ModelRequest,
        SaveTranscriptEditsRequest, TaskIdRequest, TranscribeTask, TranscribeTaskListSnapshot,
        TranscriptResult,
    },
};

#[tauri::command]
pub async fn inspect_transcription_source(
    request: InspectTranscriptionSourceRequest,
    runtime: State<'_, Arc<TranscriptionRuntime>>,
) -> Result<MediaSourceInfo, AppError> {
    runtime.inspect_source(&request.source).await
}

#[tauri::command]
pub fn list_transcribe_tasks(
    runtime: State<'_, Arc<TranscriptionRuntime>>,
) -> TranscribeTaskListSnapshot {
    runtime.manager().snapshot()
}

#[tauri::command]
pub async fn create_transcribe_task(
    request: CreateTranscribeTaskRequest,
    runtime: State<'_, Arc<TranscriptionRuntime>>,
) -> Result<CreateTranscribeTaskResult, AppError> {
    runtime.create_task(request).await
}

#[tauri::command]
pub fn control_transcribe_task(
    request: ControlTranscribeTaskRequest,
    runtime: State<'_, Arc<TranscriptionRuntime>>,
) -> Result<TranscribeTask, AppError> {
    runtime.control_task(&request.task_id, request.action)
}

#[tauri::command]
pub fn clear_completed_transcribe_tasks(
    runtime: State<'_, Arc<TranscriptionRuntime>>,
) -> Result<Vec<String>, AppError> {
    runtime.manager().clear_completed()
}

#[tauri::command]
pub fn list_asr_models(
    runtime: State<'_, Arc<TranscriptionRuntime>>,
) -> Result<AsrModelListSnapshot, AppError> {
    runtime.list_models()
}

#[tauri::command]
pub async fn download_asr_model(
    request: ModelRequest,
    runtime: State<'_, Arc<TranscriptionRuntime>>,
) -> Result<AsrModel, AppError> {
    runtime.download_model(request.model_id).await
}

#[tauri::command]
pub fn cancel_asr_model_download(
    request: ModelRequest,
    runtime: State<'_, Arc<TranscriptionRuntime>>,
) -> Result<AsrModel, AppError> {
    runtime.cancel_model_download(request.model_id)
}

#[tauri::command]
pub fn verify_asr_model(
    request: ModelRequest,
    runtime: State<'_, Arc<TranscriptionRuntime>>,
) -> Result<AsrModel, AppError> {
    runtime.verify_model(request.model_id)
}

#[tauri::command]
pub fn delete_asr_model(
    request: ModelRequest,
    runtime: State<'_, Arc<TranscriptionRuntime>>,
) -> Result<AsrModelListSnapshot, AppError> {
    runtime.delete_model(request.model_id)
}

#[tauri::command]
pub fn get_transcript_result(
    request: TaskIdRequest,
    runtime: State<'_, Arc<TranscriptionRuntime>>,
) -> Result<TranscriptResult, AppError> {
    runtime.manager().result(&request.task_id)
}

#[tauri::command]
pub fn save_transcript_edits(
    request: SaveTranscriptEditsRequest,
    runtime: State<'_, Arc<TranscriptionRuntime>>,
) -> Result<TranscriptResult, AppError> {
    runtime.manager().save_edits(
        &request.task_id,
        request.expected_revision,
        request.segments,
    )
}

#[tauri::command]
pub fn export_transcript(
    request: ExportTranscriptRequest,
    runtime: State<'_, Arc<TranscriptionRuntime>>,
) -> Result<ExportTranscriptResult, AppError> {
    runtime.export(request)
}

#[tauri::command]
pub fn open_transcript_output(
    request: TaskIdRequest,
    runtime: State<'_, Arc<TranscriptionRuntime>>,
) -> Result<(), AppError> {
    let path = runtime.output_directory(&request.task_id)?;
    tauri_plugin_opener::open_path(path, None::<&str>)
        .map_err(|_| AppError::internal("Unable to open transcript output"))
}
