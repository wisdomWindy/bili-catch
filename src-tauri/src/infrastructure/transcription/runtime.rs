use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc, Mutex, Weak,
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::{
    infrastructure::transcription::{
        ffmpeg_preprocessor::{FfmpegPreprocessRequest, FfmpegTranscriptionPreprocessor},
        fire_red_sidecar::{FireRedSidecarTranscriber, FIRE_RED_MAX_CHUNK_SECONDS},
        model_repository::{
            FileModelRepository, ModelDownloadCancellation, ReqwestModelDownloader,
        },
        whisper_sidecar::WhisperSidecarTranscriber,
    },
    models::{
        AppError, AppErrorCode, AsrModel, AsrModelId, AsrModelListSnapshot,
        CreateTranscribeTaskRequest, CreateTranscribeTaskResult, ExportTranscriptRequest,
        ExportTranscriptResult, GpuMode, MediaSourceInfo, ModelDownloadState, TranscribeTask,
        TranscribeTaskAction, TranscribeTaskStatus, TranscriptFormat, TranscriptResult,
        TranscriptionLanguage, TranscriptionOptions, TranscriptionProgress, TranscriptionSource,
    },
    services::{
        audio::ProcessControl,
        settings::SettingsManager,
        tasks::{DownloadCompletionSink, TaskManager},
        transcription::{
            cache::source_fingerprint,
            export::{choose_export_path, render_transcript, write_export_atomic},
            ports::{
                SubtitleLookup, SubtitleProvider, Transcriber, TranscriberOutput,
                TranscriberRequest,
            },
            processing::{apply_punctuation_preference, merge_segments},
            scheduler::{effective_capacity, FairPendingQueue},
            TranscriptionManager,
        },
    },
};

pub trait ModelEventSink: Send + Sync {
    fn model_changed(&self, sequence: u64, model: &AsrModel);
}

#[derive(Default)]
pub struct LateBoundTranscriptionCompletionHook {
    runtime: Mutex<Option<Weak<TranscriptionRuntime>>>,
    seen: Mutex<HashSet<String>>,
}

impl LateBoundTranscriptionCompletionHook {
    pub fn bind(&self, runtime: &Arc<TranscriptionRuntime>) {
        if let Ok(mut target) = self.runtime.lock() {
            *target = Some(Arc::downgrade(runtime));
        }
    }
}

impl DownloadCompletionSink for LateBoundTranscriptionCompletionHook {
    fn completed(&self, task: &crate::models::DownloadTask) {
        let runtime = self
            .runtime
            .lock()
            .ok()
            .and_then(|target| target.as_ref().and_then(Weak::upgrade));
        let Some(runtime) = runtime else {
            return;
        };
        let settings = runtime.settings.snapshot().values;
        if !task.auto_transcribe
            || task.output_path.is_none()
            || task.mode == crate::models::DownloadMode::VideoOnly
        {
            return;
        }
        let key = format!("download:{}:{}", task.id, task.revision);
        if !self.seen.lock().is_ok_and(|mut seen| seen.insert(key)) {
            return;
        }
        let preset = task.transcription_preset.clone();
        let request = CreateTranscribeTaskRequest {
            source: TranscriptionSource::DownloadTask {
                task_id: task.id.clone(),
                output_path: task.output_path.clone().unwrap_or_default(),
                bvid: Some(task.bvid.clone()),
                cid: Some(task.cid),
            },
            options: TranscriptionOptions {
                model_id: preset
                    .as_ref()
                    .map_or(settings.default_asr_model, |value| value.model_id),
                language: preset
                    .as_ref()
                    .map_or(settings.default_transcription_language, |value| {
                        value.language
                    }),
                output_format: preset
                    .as_ref()
                    .map_or(settings.default_transcript_format, |value| value.format),
                timestamps: preset
                    .as_ref()
                    .map_or(settings.transcription_timestamps, |value| value.timestamps)
                    || matches!(
                        preset
                            .as_ref()
                            .map_or(settings.default_transcript_format, |value| value.format),
                        TranscriptFormat::Srt | TranscriptFormat::Vtt | TranscriptFormat::Json
                    ),
                punctuation: preset
                    .as_ref()
                    .map_or(settings.transcription_punctuation, |value| {
                        value.punctuation
                    }),
                output_directory: PathBuf::from(&settings.download_directory)
                    .join("Transcribe")
                    .to_string_lossy()
                    .into_owned(),
                gpu_mode: settings.asr_gpu_mode,
            },
            force: false,
        };
        tauri::async_runtime::spawn(async move {
            if let Err(error) = runtime.create_task(request.clone()).await {
                let _ = runtime.record_automatic_failure(request, error).await;
            }
        });
    }
}

#[derive(Debug, Clone)]
pub struct TranscriptionSidecarPaths {
    pub ffmpeg: PathBuf,
    pub whisper: PathBuf,
    pub sherpa: PathBuf,
    pub fire_red_tokens: PathBuf,
}

pub struct TranscriptionRuntime {
    manager: Arc<TranscriptionManager>,
    settings: Arc<SettingsManager>,
    downloads: Arc<TaskManager>,
    sidecars: TranscriptionSidecarPaths,
    queue: Mutex<FairPendingQueue>,
    force_runs: Mutex<HashMap<String, bool>>,
    controls: Mutex<HashMap<String, ProcessControl>>,
    active: AtomicUsize,
    model_downloads: Mutex<HashMap<AsrModelId, ModelDownloadCancellation>>,
    model_sequence: Arc<AtomicU64>,
    model_events: Arc<dyn ModelEventSink>,
    subtitles: Arc<dyn SubtitleProvider>,
    id_sequence: AtomicU64,
}

impl TranscriptionRuntime {
    pub fn new(
        manager: Arc<TranscriptionManager>,
        settings: Arc<SettingsManager>,
        downloads: Arc<TaskManager>,
        sidecars: TranscriptionSidecarPaths,
        model_events: Arc<dyn ModelEventSink>,
        subtitles: Arc<dyn SubtitleProvider>,
    ) -> Self {
        Self {
            manager,
            settings,
            downloads,
            sidecars,
            queue: Mutex::new(FairPendingQueue::default()),
            force_runs: Mutex::new(HashMap::new()),
            controls: Mutex::new(HashMap::new()),
            active: AtomicUsize::new(0),
            model_downloads: Mutex::new(HashMap::new()),
            model_sequence: Arc::new(AtomicU64::new(0)),
            model_events,
            subtitles,
            id_sequence: AtomicU64::new(1),
        }
    }

    pub fn manager(&self) -> &Arc<TranscriptionManager> {
        &self.manager
    }

    pub async fn inspect_source(
        &self,
        source: &TranscriptionSource,
    ) -> Result<MediaSourceInfo, AppError> {
        let path = self.resolve_source_path(source)?;
        FfmpegTranscriptionPreprocessor::new(&self.sidecars.ffmpeg)?
            .inspect(&path, ProcessControl::new())
            .await
    }

    pub async fn create_task(
        &self,
        request: CreateTranscribeTaskRequest,
    ) -> Result<CreateTranscribeTaskResult, AppError> {
        validate_model_options(
            request.options.model_id,
            request.options.language,
            request.options.gpu_mode,
        )?;
        let source_path = self.resolve_source_path(&request.source)?;
        let source_info = FfmpegTranscriptionPreprocessor::new(&self.sidecars.ffmpeg)?
            .inspect(&source_path, ProcessControl::new())
            .await?;
        let settings = self.settings.snapshot().values;
        let repository = FileModelRepository::new(&settings.asr_model_directory);
        repository.verify(request.options.model_id, settings.default_asr_model)?;
        if request.options.gpu_mode == crate::models::GpuMode::Enabled
            && !self.sidecars.whisper.is_file()
        {
            return Err(runtime_error(
                AppErrorCode::E014,
                "SIDECAR_UNAVAILABLE",
                "The transcription engine is unavailable",
            ));
        }
        let output_directory = PathBuf::from(request.options.output_directory.trim());
        if output_directory.as_os_str().is_empty() {
            return Err(runtime_error(
                AppErrorCode::E016,
                "EXPORT_DIRECTORY_INVALID",
                "The transcript output directory is invalid",
            ));
        }
        fs::create_dir_all(&output_directory).map_err(|_| {
            runtime_error(
                AppErrorCode::E016,
                "EXPORT_DIRECTORY_UNAVAILABLE",
                "The transcript output directory is unavailable",
            )
        })?;
        let id = self.next_id("transcribe");
        let now = now_rfc3339()?;
        let task = TranscribeTask {
            id: id.clone(),
            revision: 0,
            source: request.source,
            source_info: source_info.clone(),
            options: request.options,
            status: TranscribeTaskStatus::Pending,
            progress: TranscriptionProgress {
                progress: 0.0,
                processed_seconds: 0.0,
                total_seconds: Some(source_info.duration_seconds as f64),
                speed: None,
                eta_seconds: None,
                message: String::new(),
            },
            result_id: None,
            output_path: None,
            error: None,
            cache_reused: false,
            created_at: now.clone(),
            updated_at: now,
        };
        let task = self.manager.insert(task)?;
        self.force_runs
            .lock()
            .map_err(|_| internal("TRANSCRIPTION_RUNTIME_LOCK"))?
            .insert(id.clone(), request.force);
        self.queue
            .lock()
            .map_err(|_| internal("TRANSCRIPTION_RUNTIME_LOCK"))?
            .enqueue(id);
        Ok(CreateTranscribeTaskResult {
            task,
            reused: false,
        })
    }

    async fn record_automatic_failure(
        &self,
        request: CreateTranscribeTaskRequest,
        error: AppError,
    ) -> Result<(), AppError> {
        let source_info = self.inspect_source(&request.source).await?;
        let id = self.next_id("transcribe");
        let now = now_rfc3339()?;
        let task = TranscribeTask {
            id: id.clone(),
            revision: 0,
            source: request.source,
            source_info: source_info.clone(),
            options: request.options,
            status: TranscribeTaskStatus::Pending,
            progress: TranscriptionProgress {
                progress: 0.0,
                processed_seconds: 0.0,
                total_seconds: Some(source_info.duration_seconds as f64),
                speed: None,
                eta_seconds: None,
                message: String::new(),
            },
            result_id: None,
            output_path: None,
            error: None,
            cache_reused: false,
            created_at: now.clone(),
            updated_at: now,
        };
        self.manager.insert(task)?;
        self.manager
            .transition(&id, TranscribeTaskStatus::Failed, Some(error))?;
        Ok(())
    }

    pub fn control_task(
        &self,
        task_id: &str,
        action: TranscribeTaskAction,
    ) -> Result<TranscribeTask, AppError> {
        match action {
            TranscribeTaskAction::Cancel => {
                let current = self.task(task_id)?;
                if current.status == TranscribeTaskStatus::Pending {
                    self.queue
                        .lock()
                        .map_err(|_| internal("TRANSCRIPTION_RUNTIME_LOCK"))?
                        .remove(task_id);
                    self.manager
                        .transition(task_id, TranscribeTaskStatus::Cancelled, None)
                } else {
                    if let Some(control) = self
                        .controls
                        .lock()
                        .map_err(|_| internal("TRANSCRIPTION_RUNTIME_LOCK"))?
                        .get(task_id)
                    {
                        control.request_cancel();
                    }
                    Ok(current)
                }
            }
            TranscribeTaskAction::Retry => {
                let task = self.manager.retry(task_id)?;
                self.queue
                    .lock()
                    .map_err(|_| internal("TRANSCRIPTION_RUNTIME_LOCK"))?
                    .enqueue(task_id);
                Ok(task)
            }
            TranscribeTaskAction::Delete => {
                let task = self.task(task_id)?;
                self.manager.delete(task_id)?;
                Ok(task)
            }
        }
    }

    pub fn task(&self, task_id: &str) -> Result<TranscribeTask, AppError> {
        self.manager
            .snapshot()
            .tasks
            .into_iter()
            .find(|task| task.id == task_id)
            .ok_or_else(|| internal("TASK_NOT_FOUND"))
    }

    pub fn list_models(&self) -> Result<AsrModelListSnapshot, AppError> {
        let values = self.settings.snapshot().values;
        let mut models =
            FileModelRepository::new(&values.asr_model_directory).list(values.default_asr_model)?;
        let active_downloads = self
            .model_downloads
            .lock()
            .map_err(|_| internal("MODEL_DOWNLOAD_LOCK"))?;
        for model in &mut models {
            if active_downloads.contains_key(&model.id) {
                model.download_state = ModelDownloadState::Downloading;
            }
        }
        Ok(AsrModelListSnapshot {
            sequence: self.model_sequence.load(Ordering::Acquire),
            models,
        })
    }

    pub async fn download_model(&self, model_id: AsrModelId) -> Result<AsrModel, AppError> {
        let values = self.settings.snapshot().values;
        let repository = FileModelRepository::new(&values.asr_model_directory);
        let cancellation = ModelDownloadCancellation::new();
        {
            let mut active = self
                .model_downloads
                .lock()
                .map_err(|_| internal("MODEL_DOWNLOAD_LOCK"))?;
            if active.contains_key(&model_id) {
                return Err(runtime_error(
                    AppErrorCode::E012,
                    "MODEL_DOWNLOAD_ACTIVE",
                    "The model is already downloading",
                ));
            }
            active.insert(model_id, cancellation.clone());
        }
        let downloader = ReqwestModelDownloader::new()?;
        let sink = self.model_events.clone();
        let sequence = self.model_sequence.clone();
        let default_model = values.default_asr_model;
        let progress_model = repository
            .list(default_model)?
            .into_iter()
            .find(|model| model.id == model_id)
            .ok_or_else(|| internal("MODEL_NOT_FOUND"))?;
        let mut result = downloader
            .download(
                &repository,
                model_id,
                default_model,
                &values.asr_model_source,
                cancellation.clone(),
                move |downloaded, total| {
                    let revision = sequence.fetch_add(1, Ordering::AcqRel) + 1;
                    let mut model = progress_model.clone();
                    model.size_bytes = total;
                    model.download_state = ModelDownloadState::Downloading;
                    model.downloaded_bytes = downloaded;
                    model.error = None;
                    model.revision = revision;
                    sink.model_changed(revision, &model);
                },
            )
            .await;
        self.model_downloads
            .lock()
            .map_err(|_| internal("MODEL_DOWNLOAD_LOCK"))?
            .remove(&model_id);
        let revision = self.model_sequence.fetch_add(1, Ordering::AcqRel) + 1;
        match &mut result {
            Ok(model) => {
                model.revision = revision;
                self.model_events.model_changed(revision, model);
            }
            Err(error) => {
                let mut model = repository
                    .list(default_model)?
                    .into_iter()
                    .find(|model| model.id == model_id)
                    .ok_or_else(|| internal("MODEL_NOT_FOUND"))?;
                model.download_state = if cancellation.is_cancelled() {
                    ModelDownloadState::Cancelled
                } else {
                    ModelDownloadState::Failed
                };
                model.error = Some(error.clone());
                model.revision = revision;
                self.model_events.model_changed(revision, &model);
            }
        }
        result
    }

    pub fn cancel_model_download(&self, model_id: AsrModelId) -> Result<AsrModel, AppError> {
        if let Some(cancellation) = self
            .model_downloads
            .lock()
            .map_err(|_| internal("MODEL_DOWNLOAD_LOCK"))?
            .get(&model_id)
        {
            cancellation.cancel();
        }
        self.list_models()?
            .models
            .into_iter()
            .find(|model| model.id == model_id)
            .ok_or_else(|| internal("MODEL_UNKNOWN"))
    }

    pub fn verify_model(&self, model_id: AsrModelId) -> Result<AsrModel, AppError> {
        let values = self.settings.snapshot().values;
        FileModelRepository::new(&values.asr_model_directory)
            .verify(model_id, values.default_asr_model)
    }

    pub fn delete_model(&self, model_id: AsrModelId) -> Result<AsrModelListSnapshot, AppError> {
        if self.model_in_use(model_id) {
            return Err(runtime_error(
                AppErrorCode::E017,
                "MODEL_IN_USE",
                "The model is in use",
            ));
        }
        let values = self.settings.snapshot().values;
        FileModelRepository::new(&values.asr_model_directory).delete(model_id, false)?;
        self.model_sequence.fetch_add(1, Ordering::AcqRel);
        self.list_models()
    }

    pub fn export(
        &self,
        request: ExportTranscriptRequest,
    ) -> Result<ExportTranscriptResult, AppError> {
        let result = self.manager.result(&request.task_id)?;
        if result.revision != request.result_revision {
            return Err(internal("RESULT_REVISION_CONFLICT"));
        }
        let task = self.task(&request.task_id)?;
        let timestamps = request.timestamps
            || matches!(
                request.format,
                crate::models::TranscriptFormat::Srt
                    | crate::models::TranscriptFormat::Vtt
                    | crate::models::TranscriptFormat::Json
            );
        let content = render_transcript(
            &result.segments,
            &result.detected_language,
            request.format,
            timestamps,
        )?;
        let path = choose_export_path(
            Path::new(&request.output_directory),
            &task.source_info.file_name,
            &result.detected_language,
            request.format,
        )?;
        write_export_atomic(&path, &content)?;
        Ok(ExportTranscriptResult {
            output_path: path.to_string_lossy().into_owned(),
        })
    }

    pub fn output_directory(&self, task_id: &str) -> Result<PathBuf, AppError> {
        let task = self.task(task_id)?;
        let output = PathBuf::from(
            task.output_path
                .ok_or_else(|| internal("OUTPUT_NOT_FOUND"))?,
        );
        output
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| internal("OUTPUT_DIRECTORY_NOT_FOUND"))
    }

    pub fn tick(self: &Arc<Self>) {
        let values = self.settings.snapshot().values;
        let active_downloads = self
            .downloads
            .list()
            .tasks
            .iter()
            .filter(|task| {
                matches!(
                    task.status,
                    crate::models::TaskStatus::Downloading | crate::models::TaskStatus::Processing
                )
            })
            .count();
        let capacity = effective_capacity(
            values.max_concurrent_transcriptions as usize,
            active_downloads,
        );
        while self.active.load(Ordering::Acquire) < capacity {
            let task_id = match self
                .queue
                .lock()
                .ok()
                .and_then(|mut queue| queue.acquire(capacity))
            {
                Some(task_id) => task_id,
                None => break,
            };
            self.active.fetch_add(1, Ordering::AcqRel);
            let runtime = self.clone();
            tauri::async_runtime::spawn(async move {
                runtime.run_task(task_id.clone()).await;
                if let Ok(mut queue) = runtime.queue.lock() {
                    queue.release(&task_id);
                }
                if let Ok(mut controls) = runtime.controls.lock() {
                    controls.remove(&task_id);
                }
                runtime.active.fetch_sub(1, Ordering::AcqRel);
            });
        }
    }

    async fn run_task(&self, task_id: String) {
        let control = ProcessControl::new();
        if let Ok(mut controls) = self.controls.lock() {
            controls.insert(task_id.clone(), control.clone());
        }
        let result = self.run_pipeline(&task_id, control.clone()).await;
        if let Err(error) = result {
            let current = match self.task(&task_id) {
                Ok(task) => task,
                Err(_) => return,
            };
            let cancelled =
                control.state() == crate::services::audio::ProcessControlState::CancelRequested;
            let target = if cancelled {
                TranscribeTaskStatus::Cancelled
            } else {
                TranscribeTaskStatus::Failed
            };
            let _ = self
                .manager
                .transition(&task_id, target, (!cancelled).then_some(error));
            if current.status == TranscribeTaskStatus::Saving && cancelled {
                let _ = self.manager.transition(
                    &task_id,
                    TranscribeTaskStatus::Failed,
                    Some(internal("CANCEL_DURING_SAVE")),
                );
            }
        }
    }

    async fn run_pipeline(&self, task_id: &str, control: ProcessControl) -> Result<(), AppError> {
        let task = self.task(task_id)?;
        let source_path = self.resolve_source_path(&task.source)?;
        let metadata = fs::metadata(&source_path).map_err(|_| {
            runtime_error(
                AppErrorCode::E013,
                "SOURCE_UNAVAILABLE",
                "The media source is unavailable",
            )
        })?;
        let modified = metadata
            .modified()
            .ok()
            .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
            .map(|value| value.as_millis())
            .unwrap_or_default();
        let fingerprint = source_fingerprint(
            &source_path,
            metadata.len(),
            modified,
            task.options.model_id,
            task.options.language,
            task.options.punctuation,
        );
        let force = self
            .force_runs
            .lock()
            .ok()
            .and_then(|mut values| values.remove(task_id))
            .unwrap_or(false);
        let settings = self.settings.snapshot().values;
        self.manager
            .transition(task_id, TranscribeTaskStatus::Preprocessing, None)?;
        if !force {
            if let Some(mut cached) = self.manager.find_cached_result(&fingerprint) {
                self.manager
                    .transition(task_id, TranscribeTaskStatus::LoadingModel, None)?;
                self.manager
                    .transition(task_id, TranscribeTaskStatus::Transcribing, None)?;
                self.manager
                    .transition(task_id, TranscribeTaskStatus::Processing, None)?;
                self.manager
                    .transition(task_id, TranscribeTaskStatus::Saving, None)?;
                cached.id = self.next_id("result");
                cached.task_id = task_id.into();
                cached.revision = 0;
                let path = self.write_primary_output(&task, &cached)?;
                self.manager.complete_with_result(
                    task_id,
                    cached,
                    path.to_string_lossy().into_owned(),
                    true,
                )?;
                return Ok(());
            }
        }
        if settings.prefer_platform_subtitles {
            if let TranscriptionSource::DownloadTask {
                bvid: Some(bvid),
                cid: Some(cid),
                ..
            } = &task.source
            {
                if let SubtitleLookup::Available(output) = self.subtitles.lookup(bvid, *cid).await {
                    self.manager
                        .transition(task_id, TranscribeTaskStatus::LoadingModel, None)?;
                    self.manager
                        .transition(task_id, TranscribeTaskStatus::Transcribing, None)?;
                    self.manager
                        .transition(task_id, TranscribeTaskStatus::Processing, None)?;
                    return self.complete_recognition(task_id, &task, fingerprint, output, false);
                }
            }
        }
        let preprocessor = FfmpegTranscriptionPreprocessor::new(&self.sidecars.ffmpeg)?;
        let preprocess_request = FfmpegPreprocessRequest {
            task_id: task_id.into(),
            run_revision: task.revision,
            source_path,
            duration_seconds: task.source_info.duration_seconds,
            temporary_directory: PathBuf::from(&settings.temporary_directory)
                .join("bilicatch-transcribe"),
            silence_points: Vec::new(),
        };
        let engine = transcription_engine(task.options.model_id);
        let parts = match engine {
            TranscriptionEngine::Whisper => {
                preprocessor
                    .preprocess(preprocess_request, control.clone())
                    .await?
            }
            TranscriptionEngine::FireRed => {
                preprocessor
                    .preprocess_with_chunk_limit(
                        preprocess_request,
                        control.clone(),
                        FIRE_RED_MAX_CHUNK_SECONDS,
                    )
                    .await?
            }
        };
        self.manager
            .transition(task_id, TranscribeTaskStatus::LoadingModel, None)?;
        let model = FileModelRepository::new(&settings.asr_model_directory)
            .verify(task.options.model_id, settings.default_asr_model)?;
        let model_path = PathBuf::from(model.path.ok_or_else(|| {
            runtime_error(
                AppErrorCode::E011,
                "MODEL_UNAVAILABLE",
                "The model is unavailable",
            )
        })?);
        let transcriber: Box<dyn Transcriber> = match engine {
            TranscriptionEngine::Whisper => {
                Box::new(WhisperSidecarTranscriber::new(&self.sidecars.whisper)?)
            }
            TranscriptionEngine::FireRed => Box::new(FireRedSidecarTranscriber::new(
                &self.sidecars.sherpa,
                &self.sidecars.fire_red_tokens,
            )?),
        };
        self.manager
            .transition(task_id, TranscribeTaskStatus::Transcribing, None)?;
        let mut chunks = Vec::new();
        let mut detected_language = None;
        let total_seconds = task.source_info.duration_seconds.max(1) as f64;
        let started = Instant::now();
        for (index, part) in parts.parts().iter().enumerate() {
            let part_start = part.offset_ms as f64 / 1000.0;
            let part_end = parts
                .parts()
                .get(index + 1)
                .map_or(total_seconds, |next| next.offset_ms as f64 / 1000.0);
            let part_duration = (part_end - part_start).max(0.0);
            let manager = self.manager.clone();
            let progress_task_id = task_id.to_owned();
            let last_percent = Arc::new(AtomicUsize::new(usize::MAX));
            let progress_last_percent = last_percent.clone();
            let progress = Arc::new(move |part_progress: f64| {
                let processed_seconds =
                    (part_start + part_duration * part_progress.clamp(0.0, 1.0)).min(total_seconds);
                let overall = (processed_seconds / total_seconds).clamp(0.0, 1.0);
                let percent = (overall * 100.0).floor() as usize;
                if progress_last_percent.swap(percent, Ordering::AcqRel) == percent {
                    return;
                }
                let elapsed = started.elapsed().as_secs_f64().max(0.001);
                let speed = processed_seconds / elapsed;
                let eta_seconds = (speed > 0.0)
                    .then(|| ((total_seconds - processed_seconds).max(0.0) / speed).ceil() as u64);
                let _ = manager.update_progress(
                    &progress_task_id,
                    TranscriptionProgress {
                        progress: overall,
                        processed_seconds,
                        total_seconds: Some(total_seconds),
                        speed: Some(speed),
                        eta_seconds,
                        message: "Transcribing".into(),
                    },
                );
            });
            let output = transcriber
                .transcribe(
                    TranscriberRequest {
                        model_path: model_path.clone(),
                        audio_path: part.path.clone(),
                        output_base: part.path.with_extension("transcript"),
                        language: task.options.language,
                        gpu_mode: task.options.gpu_mode,
                        offset_ms: part.offset_ms,
                    },
                    control.clone(),
                    progress,
                )
                .await?;
            if detected_language.is_none() {
                detected_language = Some(output.detected_language.clone());
            }
            chunks.push(output.segments);
        }
        self.manager
            .transition(task_id, TranscribeTaskStatus::Processing, None)?;
        self.complete_recognition(
            task_id,
            &task,
            fingerprint,
            TranscriberOutput {
                detected_language: detected_language.unwrap_or_else(|| "und".into()),
                segments: merge_segments(chunks),
            },
            false,
        )
    }

    fn complete_recognition(
        &self,
        task_id: &str,
        task: &TranscribeTask,
        fingerprint: String,
        output: TranscriberOutput,
        cache_reused: bool,
    ) -> Result<(), AppError> {
        let mut segments = output.segments;
        for segment in &mut segments {
            segment.text = apply_punctuation_preference(&segment.text, task.options.punctuation);
        }
        if segments.is_empty() {
            return Err(runtime_error(
                AppErrorCode::E015,
                "NO_SPEECH_DETECTED",
                "No speech was detected",
            ));
        }
        let plain_text = segments
            .iter()
            .map(|segment| segment.text.trim())
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        let detected_language = output.detected_language;
        let now = now_rfc3339()?;
        let result = TranscriptResult {
            id: self.next_id("result"),
            task_id: task_id.into(),
            revision: 0,
            source_fingerprint: fingerprint,
            model_id: task.options.model_id,
            requested_language: task.options.language,
            detected_language,
            segments,
            plain_text,
            created_at: now.clone(),
            updated_at: now,
        };
        self.manager
            .transition(task_id, TranscribeTaskStatus::Saving, None)?;
        let output_path = self.write_primary_output(&task, &result)?;
        self.manager.complete_with_result(
            task_id,
            result,
            output_path.to_string_lossy().into_owned(),
            cache_reused,
        )?;
        Ok(())
    }

    fn write_primary_output(
        &self,
        task: &TranscribeTask,
        result: &TranscriptResult,
    ) -> Result<PathBuf, AppError> {
        let path = choose_export_path(
            Path::new(&task.options.output_directory),
            &task.source_info.file_name,
            &result.detected_language,
            task.options.output_format,
        )?;
        let content = render_transcript(
            &result.segments,
            &result.detected_language,
            task.options.output_format,
            task.options.timestamps,
        )?;
        write_export_atomic(&path, &content)?;
        Ok(path)
    }

    fn resolve_source_path(&self, source: &TranscriptionSource) -> Result<PathBuf, AppError> {
        match source {
            TranscriptionSource::LocalFile { path } => Ok(PathBuf::from(path)),
            TranscriptionSource::DownloadTask { task_id, .. } => {
                let task = self
                    .downloads
                    .list()
                    .tasks
                    .into_iter()
                    .find(|task| task.id == *task_id)
                    .ok_or_else(|| internal("DOWNLOAD_TASK_NOT_FOUND"))?;
                if task.status != crate::models::TaskStatus::Completed {
                    return Err(runtime_error(
                        AppErrorCode::E013,
                        "DOWNLOAD_NOT_COMPLETED",
                        "The download is not completed",
                    ));
                }
                task.output_path
                    .map(PathBuf::from)
                    .ok_or_else(|| internal("DOWNLOAD_OUTPUT_NOT_FOUND"))
            }
        }
    }

    fn model_in_use(&self, model_id: AsrModelId) -> bool {
        self.manager.snapshot().tasks.iter().any(|task| {
            task.options.model_id == model_id
                && matches!(
                    task.status,
                    TranscribeTaskStatus::Preprocessing
                        | TranscribeTaskStatus::LoadingModel
                        | TranscribeTaskStatus::Transcribing
                        | TranscribeTaskStatus::Processing
                        | TranscribeTaskStatus::Saving
                )
        })
    }

    fn next_id(&self, prefix: &str) -> String {
        let sequence = self.id_sequence.fetch_add(1, Ordering::AcqRel);
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|value| value.as_millis())
            .unwrap_or_default();
        format!("{prefix}-{millis}-{sequence}")
    }
}

fn now_rfc3339() -> Result<String, AppError> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|_| internal("SYSTEM_CLOCK_UNAVAILABLE"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TranscriptionEngine {
    Whisper,
    FireRed,
}

fn transcription_engine(model_id: AsrModelId) -> TranscriptionEngine {
    match model_id {
        AsrModelId::FireRedAsr2 => TranscriptionEngine::FireRed,
        AsrModelId::Tiny
        | AsrModelId::Base
        | AsrModelId::Small
        | AsrModelId::Medium
        | AsrModelId::LargeV3 => TranscriptionEngine::Whisper,
    }
}

fn validate_model_options(
    model_id: AsrModelId,
    language: TranscriptionLanguage,
    gpu_mode: GpuMode,
) -> Result<(), AppError> {
    if model_id != AsrModelId::FireRedAsr2 {
        return Ok(());
    }
    if matches!(
        language,
        TranscriptionLanguage::Ja | TranscriptionLanguage::Ko
    ) {
        return Err(runtime_error(
            AppErrorCode::E014,
            "MODEL_LANGUAGE_UNSUPPORTED",
            "FireRedASR2 supports Chinese and English transcription",
        ));
    }
    if gpu_mode == GpuMode::Enabled {
        return Err(runtime_error(
            AppErrorCode::E014,
            "GPU_UNAVAILABLE",
            "FireRedASR2 currently uses the bundled CPU runtime",
        ));
    }
    Ok(())
}

fn internal(details: &str) -> AppError {
    runtime_error(
        AppErrorCode::Internal,
        details,
        "The transcription service is unavailable",
    )
}

fn runtime_error(code: AppErrorCode, details: &str, message: &str) -> AppError {
    AppError {
        code,
        message: message.into(),
        details: Some(details.into()),
    }
}

#[cfg(test)]
mod engine_tests {
    use super::{transcription_engine, validate_model_options, TranscriptionEngine};
    use crate::models::{AsrModelId, GpuMode, TranscriptionLanguage};

    #[test]
    fn fire_red_model_routes_to_sherpa_and_whisper_models_stay_on_whisper() {
        assert_eq!(
            transcription_engine(AsrModelId::FireRedAsr2),
            TranscriptionEngine::FireRed
        );
        assert_eq!(
            transcription_engine(AsrModelId::Base),
            TranscriptionEngine::Whisper
        );
    }

    #[test]
    fn fire_red_rejects_unsupported_languages_and_forced_gpu_before_execution() {
        let language_error = validate_model_options(
            AsrModelId::FireRedAsr2,
            TranscriptionLanguage::Ja,
            GpuMode::Auto,
        )
        .unwrap_err();
        assert_eq!(
            language_error.details.as_deref(),
            Some("MODEL_LANGUAGE_UNSUPPORTED")
        );

        let gpu_error = validate_model_options(
            AsrModelId::FireRedAsr2,
            TranscriptionLanguage::Zh,
            GpuMode::Enabled,
        )
        .unwrap_err();
        assert_eq!(gpu_error.details.as_deref(), Some("GPU_UNAVAILABLE"));
    }
}
