use std::{fs::OpenOptions, io::Write, sync::Arc, time::Instant};

use async_trait::async_trait;
use reqwest::{
    header::{CONTENT_RANGE, ETAG, LOCATION, RANGE, REFERER},
    redirect::Policy,
    Client, Response, StatusCode,
};
use url::Url;

use crate::{
    infrastructure::{audio::AudioWorkspace, bilibili::validate_media_url},
    models::{AppError, AppErrorCode},
    services::download::{
        media_source_identity, validate_media_source_kind, ByteDownloaderPort, DownloadControl,
        DownloadControlState, DownloadOutcome, MediaDownloadRequest, MediaKind,
        MediaWorkspacePaths, ProgressSink,
    },
};

use super::support::{
    download_io_error, header_text, network_error, network_failure, parse_content_range,
    progress_snapshot,
};

type UrlValidator = Arc<dyn Fn(&str) -> Result<Url, AppError> + Send + Sync>;

const MEDIA_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct MediaCheckpoint {
    schema_version: u32,
    task_id: String,
    source_identity_hash: String,
    completed_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    total_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    etag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    track_kind: Option<MediaKind>,
}

pub struct HttpByteDownloader {
    client: Client,
    _workspace: Arc<dyn AudioWorkspace>,
    validate_url: UrlValidator,
}

impl HttpByteDownloader {
    pub fn new(workspace: Arc<dyn AudioWorkspace>) -> Result<Self, AppError> {
        Self::with_url_validator(workspace, Arc::new(validate_media_url))
    }

    fn with_url_validator(
        workspace: Arc<dyn AudioWorkspace>,
        validate_url: UrlValidator,
    ) -> Result<Self, AppError> {
        let client = Client::builder()
            .redirect(Policy::none())
            .connect_timeout(std::time::Duration::from_secs(8))
            .timeout(std::time::Duration::from_secs(30))
            .user_agent(MEDIA_USER_AGENT)
            .build()
            .map_err(|_| AppError::internal("Unable to initialize the media downloader"))?;
        Ok(Self {
            client,
            _workspace: workspace,
            validate_url,
        })
    }

    pub(super) async fn request(&self, value: &str, offset: u64) -> Result<Response, AppError> {
        let mut url = (self.validate_url)(value)?;
        for redirect_count in 0..=5 {
            let mut request = self
                .client
                .get(url.clone())
                .header(REFERER, "https://www.bilibili.com/");
            if offset > 0 {
                request = request.header(RANGE, format!("bytes={offset}-"));
            }
            let response = request.send().await.map_err(network_error)?;
            if !response.status().is_redirection() {
                return Ok(response);
            }
            if redirect_count == 5 {
                return Err(network_failure(
                    "The media source redirected too many times",
                ));
            }
            let location = response
                .headers()
                .get(LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or_else(|| network_failure("The media redirect is invalid"))?;
            let next = url
                .join(location)
                .map_err(|_| network_failure("The media redirect is invalid"))?;
            url = (self.validate_url)(next.as_str())?;
        }
        Err(network_failure("The media source could not be reached"))
    }

    async fn download_from_url(
        &self,
        value: &str,
        request: &MediaDownloadRequest,
        control: &DownloadControl,
        progress: &dyn ProgressSink,
        identity: &str,
    ) -> Result<DownloadOutcome, AppError> {
        let checkpoint = load_checkpoint(&request.workspace.checkpoint_path, &request, identity)?;
        let mut offset = checkpoint
            .as_ref()
            .map(|checkpoint| checkpoint.completed_bytes)
            .unwrap_or(0);
        let mut restarted = false;

        loop {
            match control.state() {
                DownloadControlState::PauseRequested => return Ok(DownloadOutcome::Paused),
                DownloadControlState::CancelRequested => return Ok(DownloadOutcome::Cancelled),
                DownloadControlState::Running => {}
            }
            let mut response = self.request(value, offset).await?;
            let status = response.status();
            let response_etag = header_text(&response, ETAG);
            let total;

            match status {
                StatusCode::PARTIAL_CONTENT => {
                    let range = header_text(&response, CONTENT_RANGE)
                        .and_then(|value| parse_content_range(&value))
                        .ok_or_else(|| network_failure("The media range response is invalid"))?;
                    if range.start != Some(offset) {
                        return Err(network_failure("The media range response is inconsistent"));
                    }
                    total = Some(range.total);
                    if request
                        .source
                        .content_length
                        .is_some_and(|value| value != range.total)
                        || checkpoint
                            .as_ref()
                            .and_then(|value| value.total_bytes)
                            .is_some_and(|value| value != range.total)
                    {
                        if offset > 0 && !restarted {
                            discard_download(&request.workspace);
                            offset = 0;
                            restarted = true;
                            continue;
                        }
                        return Err(network_failure("The media length changed"));
                    }
                }
                StatusCode::OK if offset > 0 => {
                    if restarted {
                        return Err(network_failure("The media server ignored the restart"));
                    }
                    discard_download(&request.workspace);
                    offset = 0;
                    restarted = true;
                    continue;
                }
                StatusCode::OK => {
                    if request
                        .source
                        .content_length
                        .zip(response.content_length())
                        .is_some_and(|(expected, actual)| expected != actual)
                    {
                        return Err(network_failure("The media response length changed"));
                    }
                    total = request.source.content_length.or(response.content_length());
                }
                StatusCode::RANGE_NOT_SATISFIABLE => {
                    let complete = checkpoint.as_ref().is_some_and(|checkpoint| {
                        checkpoint.total_bytes == Some(checkpoint.completed_bytes)
                            && std::fs::metadata(&request.workspace.source_path)
                                .is_ok_and(|item| item.len() == checkpoint.completed_bytes)
                    });
                    if complete {
                        return Ok(DownloadOutcome::Completed);
                    }
                    if restarted {
                        return Err(network_failure("The media range could not be recovered"));
                    }
                    discard_download(&request.workspace);
                    offset = 0;
                    restarted = true;
                    continue;
                }
                StatusCode::UNAUTHORIZED => {
                    return Err(AppError::new(
                        AppErrorCode::E005,
                        "Authentication is required for this media source",
                    ))
                }
                StatusCode::FORBIDDEN => {
                    return Err(AppError::new(
                        AppErrorCode::E006,
                        "The current account cannot access this media source",
                    ))
                }
                StatusCode::REQUEST_TIMEOUT
                | StatusCode::PRECONDITION_FAILED
                | StatusCode::TOO_MANY_REQUESTS => {
                    return Err(network_failure(
                        "The media CDN temporarily rejected the request",
                    ))
                }
                status if status.is_client_error() => {
                    return Err(AppError::new(
                        AppErrorCode::E004,
                        "The requested media source is unavailable",
                    ))
                }
                _ => return Err(network_failure("The media server is unavailable")),
            }

            if request
                .source
                .etag
                .as_deref()
                .zip(response_etag.as_deref())
                .is_some_and(|(expected, actual)| expected != actual)
            {
                if offset > 0 && !restarted {
                    discard_download(&request.workspace);
                    offset = 0;
                    restarted = true;
                    continue;
                }
                return Err(network_failure("The media source changed"));
            }

            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .append(offset > 0)
                .truncate(offset == 0)
                .open(&request.workspace.source_path)
                .map_err(download_io_error)?;
            let started = Instant::now();
            while let Some(chunk) = response.chunk().await.map_err(network_error)? {
                match control.state() {
                    DownloadControlState::PauseRequested => {
                        file.flush().map_err(download_io_error)?;
                        return Ok(DownloadOutcome::Paused);
                    }
                    DownloadControlState::CancelRequested => {
                        file.flush().map_err(download_io_error)?;
                        return Ok(DownloadOutcome::Cancelled);
                    }
                    DownloadControlState::Running => {}
                }
                file.write_all(&chunk).map_err(download_io_error)?;
                file.flush().map_err(download_io_error)?;
                offset = offset.saturating_add(chunk.len() as u64);
                let checkpoint = MediaCheckpoint {
                    schema_version: 1,
                    task_id: request.task_id.clone(),
                    source_identity_hash: identity.to_owned(),
                    completed_bytes: offset,
                    total_bytes: total,
                    etag: response_etag
                        .clone()
                        .or_else(|| request.source.etag.clone()),
                    track_kind: Some(request.source.kind),
                };
                save_checkpoint(&request.workspace.checkpoint_path, &checkpoint)?;
                progress.report(progress_snapshot(offset, total, started));
            }
            file.flush().map_err(download_io_error)?;
            if total.is_some_and(|expected| expected != offset) {
                return Err(network_failure(
                    "The media response ended before completion",
                ));
            }
            return Ok(DownloadOutcome::Completed);
        }
    }
}

fn load_checkpoint(
    path: &std::path::Path,
    request: &MediaDownloadRequest,
    identity: &str,
) -> Result<Option<MediaCheckpoint>, AppError> {
    if !path.exists() {
        return Ok(None);
    }
    let loaded = std::fs::read(path)
        .map_err(download_io_error)
        .and_then(|bytes| {
            serde_json::from_slice::<MediaCheckpoint>(&bytes)
                .map_err(|_| network_failure("The media checkpoint is invalid"))
        })?;
    let compatible = loaded.schema_version == 1
        && loaded.task_id == request.task_id
        && loaded.source_identity_hash == identity
        && loaded
            .total_bytes
            .is_none_or(|total| loaded.completed_bytes <= total)
        && request
            .source
            .content_length
            .is_none_or(|total| loaded.total_bytes == Some(total))
        && request
            .source
            .etag
            .as_deref()
            .is_none_or(|etag| loaded.etag.as_deref() == Some(etag))
        && std::fs::metadata(&request.workspace.source_path)
            .map(|item| item.len())
            .ok()
            == Some(loaded.completed_bytes);
    if compatible {
        Ok(Some(loaded))
    } else {
        discard_download(&request.workspace);
        Ok(None)
    }
}

fn save_checkpoint(path: &std::path::Path, checkpoint: &MediaCheckpoint) -> Result<(), AppError> {
    let next = path.with_extension("json.next");
    std::fs::write(
        &next,
        serde_json::to_vec(checkpoint)
            .map_err(|_| network_failure("The media checkpoint is invalid"))?,
    )
    .map_err(download_io_error)?;
    if path.exists() {
        std::fs::remove_file(path).map_err(download_io_error)?;
    }
    std::fs::rename(next, path).map_err(download_io_error)
}

fn discard_download(paths: &MediaWorkspacePaths) {
    for path in [&paths.source_path, &paths.checkpoint_path] {
        if path.exists() {
            let _ = std::fs::remove_file(path);
        }
    }
}

#[async_trait]
impl ByteDownloaderPort for HttpByteDownloader {
    async fn download(
        &self,
        request: MediaDownloadRequest,
        control: DownloadControl,
        progress: &dyn ProgressSink,
    ) -> Result<DownloadOutcome, AppError> {
        if !(1..=32).contains(&request.connection_count) {
            return Err(AppError::internal(
                "The download connection count is invalid",
            ));
        }
        validate_media_source_kind(&request.source)?;
        match control.state() {
            DownloadControlState::PauseRequested => return Ok(DownloadOutcome::Paused),
            DownloadControlState::CancelRequested => return Ok(DownloadOutcome::Cancelled),
            DownloadControlState::Running => {}
        }
        let identity = media_source_identity(&request.bvid, request.cid, &request.source);
        let urls = std::iter::once(&request.source.primary_url)
            .chain(request.source.backup_urls.iter())
            .collect::<Vec<_>>();
        let mut last_network_error = None;
        for url in urls {
            match self
                .download_from_url(url, &request, &control, progress, &identity)
                .await
            {
                Err(error) if error.code == AppErrorCode::E009 => last_network_error = Some(error),
                result => return result,
            }
        }
        Err(last_network_error.unwrap_or_else(|| {
            AppError::new(AppErrorCode::E004, "The media source has no usable URL")
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        io::{Read, Write},
        net::TcpListener,
        sync::{Arc, Mutex},
        thread,
    };

    use crate::{
        infrastructure::audio::{
            AudioWorkspace, CheckpointV1, FsAudioWorkspace, WorkspacePaths, WorkspaceRequest,
        },
        models::AppErrorCode,
        services::download::{
            media_source_identity, ByteDownloaderPort, CoverDownloaderPort, DownloadControl,
            DownloadOutcome, DownloadProgress, MediaDownloadRequest, MediaKind,
            MediaSourceCandidate, MediaWorkspacePaths, ProgressSink,
        },
    };

    use super::HttpByteDownloader;

    struct RecordingProgress(Mutex<Vec<DownloadProgress>>);

    impl ProgressSink for RecordingProgress {
        fn report(&self, progress: DownloadProgress) {
            self.0.lock().unwrap().push(progress);
        }
    }

    fn serve(responses: Vec<&'static str>) -> (String, Arc<Mutex<Vec<String>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured = requests.clone();
        thread::spawn(move || {
            for response in responses {
                let (mut stream, _) = listener.accept().unwrap();
                let mut bytes = vec![0; 4096];
                let count = stream.read(&mut bytes).unwrap();
                captured
                    .lock()
                    .unwrap()
                    .push(String::from_utf8_lossy(&bytes[..count]).into_owned());
                stream.write_all(response.as_bytes()).unwrap();
                stream.flush().unwrap();
            }
        });
        (format!("http://{address}/audio.m4s"), requests)
    }

    fn source(url: String) -> MediaSourceCandidate {
        MediaSourceCandidate {
            id: 30216,
            kind: MediaKind::Audio,
            bandwidth: 64_000,
            primary_url: url,
            backup_urls: vec![],
            mime_type: "audio/mp4".into(),
            codecs: "mp4a.40.2".into(),
            content_length: Some(11),
            etag: Some("fixture".into()),
        }
    }

    fn setup(url: String) -> (tempfile::TempDir, tempfile::TempDir, MediaDownloadRequest) {
        let temp = tempfile::tempdir().unwrap();
        let output = tempfile::tempdir().unwrap();
        let workspace = FsAudioWorkspace;
        let paths = workspace
            .prepare(&WorkspaceRequest {
                task_id: "task-1".into(),
                attempt_id: "attempt-1".into(),
                temporary_root: temp.path().into(),
                output_directory: output.path().into(),
                final_file_name: "Track.m4a".into(),
            })
            .unwrap();
        (
            temp,
            output,
            MediaDownloadRequest {
                task_id: "task-1".into(),
                bvid: "BV1xx411c7BF".into(),
                cid: 1001,
                source: source(url),
                workspace: MediaWorkspacePaths {
                    source_path: paths.source_path,
                    checkpoint_path: paths.checkpoint_path,
                },
                connection_count: 4,
            },
        )
    }

    fn checkpoint(request: &MediaDownloadRequest, completed_bytes: u64) -> CheckpointV1 {
        CheckpointV1 {
            schema_version: 1,
            task_id: request.task_id.clone(),
            source_identity_hash: media_source_identity(
                &request.bvid,
                request.cid,
                &request.source,
            ),
            completed_bytes,
            total_bytes: request.source.content_length,
            etag: request.source.etag.clone(),
        }
    }

    fn audio_paths(paths: &MediaWorkspacePaths) -> WorkspacePaths {
        WorkspacePaths {
            temporary_root: paths
                .source_path
                .parent()
                .and_then(|value| value.parent())
                .unwrap_or_else(|| std::path::Path::new("."))
                .to_path_buf(),
            output_directory: paths
                .source_path
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .to_path_buf(),
            source_path: paths.source_path.clone(),
            cover_path: paths.source_path.with_file_name("cover.jpg"),
            checkpoint_path: paths.checkpoint_path.clone(),
            processed_path: paths.source_path.with_file_name("processed.mp3"),
            final_path: paths.source_path.with_file_name("final.mp3"),
            task_root: paths
                .source_path
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .to_path_buf(),
        }
    }

    fn downloader() -> HttpByteDownloader {
        HttpByteDownloader::with_url_validator(
            Arc::new(FsAudioWorkspace),
            Arc::new(|value| {
                url::Url::parse(value)
                    .map_err(|_| crate::models::AppError::internal("invalid fixture URL"))
            }),
        )
        .unwrap()
    }

    #[test]
    fn sends_browser_media_headers_with_every_media_request() {
        tauri::async_runtime::block_on(async {
            let (url, requests) = serve(vec![
                "HTTP/1.1 302 Found\r\nLocation: /redirected.m4s\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                "HTTP/1.1 206 Partial Content\r\nContent-Length: 1\r\nContent-Range: bytes 0-0/1\r\nConnection: close\r\n\r\nx",
            ]);

            let response = downloader().request(&url, 0).await.unwrap();

            assert_eq!(response.status(), reqwest::StatusCode::PARTIAL_CONTENT);
            let requests = requests.lock().unwrap();
            assert_eq!(requests.len(), 2);
            for request in requests.iter() {
                let request = request.to_ascii_lowercase();
                assert!(request.contains("referer: https://www.bilibili.com/\r\n"));
                assert!(request.contains(
                    "user-agent: mozilla/5.0 (windows nt 10.0; win64; x64) applewebkit/537.36"
                ));
                assert!(request.contains("chrome/"));
                assert!(request.contains("safari/537.36"));
            }
        });
    }

    #[test]
    fn retries_a_backup_url_after_cdn_precondition_rejection() {
        tauri::async_runtime::block_on(async {
            let (url, requests) = serve(vec![
                "HTTP/1.1 412 Precondition Failed\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                "HTTP/1.1 200 OK\r\nContent-Length: 11\r\nConnection: close\r\n\r\nhello world",
            ]);
            let (_temp, _output, mut request) = setup(url.clone());
            request.source.backup_urls = vec![format!("{url}?backup=1")];

            let outcome = downloader()
                .download(
                    request.clone(),
                    DownloadControl::new(),
                    &RecordingProgress(Mutex::new(Vec::new())),
                )
                .await
                .unwrap();

            assert_eq!(outcome, DownloadOutcome::Completed);
            assert_eq!(
                fs::read(&request.workspace.source_path).unwrap(),
                b"hello world"
            );
            assert_eq!(requests.lock().unwrap().len(), 2);
        });
    }

    #[test]
    fn resumes_with_a_validated_content_range_and_monotonic_progress() {
        tauri::async_runtime::block_on(async {
            let (url, requests) = serve(vec![
                "HTTP/1.1 206 Partial Content\r\nContent-Length: 5\r\nContent-Range: bytes 6-10/11\r\nETag: fixture\r\nConnection: close\r\n\r\nworld",
            ]);
            let (_temp, _output, request) = setup(url);
            fs::write(&request.workspace.source_path, b"hello ").unwrap();
            FsAudioWorkspace
                .save_checkpoint(&audio_paths(&request.workspace), &checkpoint(&request, 6))
                .unwrap();
            let progress = RecordingProgress(Mutex::new(Vec::new()));

            let result = downloader()
                .download(request.clone(), DownloadControl::new(), &progress)
                .await
                .unwrap();

            assert_eq!(result, DownloadOutcome::Completed);
            assert_eq!(
                fs::read(&request.workspace.source_path).unwrap(),
                b"hello world"
            );
            assert!(requests.lock().unwrap()[0]
                .to_ascii_lowercase()
                .contains("range: bytes=6-"));
            assert_eq!(
                progress.0.lock().unwrap().last().unwrap().downloaded_bytes,
                11
            );
        });
    }

    #[test]
    fn restarts_once_when_a_server_ignores_the_resume_range() {
        tauri::async_runtime::block_on(async {
            let response = "HTTP/1.1 200 OK\r\nContent-Length: 11\r\nETag: fixture\r\nConnection: close\r\n\r\nhello world";
            let (url, requests) = serve(vec![response, response]);
            let (_temp, _output, request) = setup(url);
            fs::write(&request.workspace.source_path, b"hello ").unwrap();
            FsAudioWorkspace
                .save_checkpoint(&audio_paths(&request.workspace), &checkpoint(&request, 6))
                .unwrap();

            let result = downloader()
                .download(
                    request.clone(),
                    DownloadControl::new(),
                    &RecordingProgress(Mutex::new(Vec::new())),
                )
                .await
                .unwrap();

            assert_eq!(result, DownloadOutcome::Completed);
            assert_eq!(
                fs::read(&request.workspace.source_path).unwrap(),
                b"hello world"
            );
            let requests = requests.lock().unwrap();
            assert!(requests[0].to_ascii_lowercase().contains("range: bytes=6-"));
            assert!(!requests[1].to_ascii_lowercase().contains("range:"));
        });
    }

    #[test]
    fn restarts_once_when_the_resume_etag_changes() {
        tauri::async_runtime::block_on(async {
            let changed = "HTTP/1.1 206 Partial Content\r\nContent-Length: 5\r\nContent-Range: bytes 6-10/11\r\nETag: changed\r\nConnection: close\r\n\r\nworld";
            let complete = "HTTP/1.1 200 OK\r\nContent-Length: 11\r\nETag: fixture\r\nConnection: close\r\n\r\nhello world";
            let (url, requests) = serve(vec![changed, complete]);
            let (_temp, _output, request) = setup(url);
            fs::write(&request.workspace.source_path, b"hello ").unwrap();
            FsAudioWorkspace
                .save_checkpoint(&audio_paths(&request.workspace), &checkpoint(&request, 6))
                .unwrap();

            let result = downloader()
                .download(
                    request.clone(),
                    DownloadControl::new(),
                    &RecordingProgress(Mutex::new(Vec::new())),
                )
                .await
                .unwrap();

            assert_eq!(result, DownloadOutcome::Completed);
            assert_eq!(
                fs::read(&request.workspace.source_path).unwrap(),
                b"hello world"
            );
            let requests = requests.lock().unwrap();
            assert!(requests[0].to_ascii_lowercase().contains("range: bytes=6-"));
            assert!(!requests[1].to_ascii_lowercase().contains("range:"));
        });
    }

    #[test]
    fn uses_one_plain_get_when_range_support_has_not_been_proven() {
        tauri::async_runtime::block_on(async {
            let response = "HTTP/1.1 200 OK\r\nContent-Length: 11\r\nETag: fixture\r\nConnection: close\r\n\r\nhello world";
            let (url, requests) = serve(vec![response]);
            let (_temp, _output, request) = setup(url);
            assert_eq!(request.connection_count, 4);

            let result = downloader()
                .download(
                    request,
                    DownloadControl::new(),
                    &RecordingProgress(Mutex::new(Vec::new())),
                )
                .await
                .unwrap();

            assert_eq!(result, DownloadOutcome::Completed);
            let requests = requests.lock().unwrap();
            assert_eq!(requests.len(), 1);
            assert!(!requests[0].to_ascii_lowercase().contains("range:"));
        });
    }

    #[test]
    fn rejects_a_malformed_partial_response_as_retryable_network_failure() {
        tauri::async_runtime::block_on(async {
            let (url, _) = serve(vec![
                "HTTP/1.1 206 Partial Content\r\nContent-Length: 5\r\nContent-Range: bytes 5-9/11\r\nConnection: close\r\n\r\nworld",
            ]);
            let (_temp, _output, request) = setup(url);
            fs::write(&request.workspace.source_path, b"hello ").unwrap();
            FsAudioWorkspace
                .save_checkpoint(&audio_paths(&request.workspace), &checkpoint(&request, 6))
                .unwrap();

            let error = downloader()
                .download(
                    request,
                    DownloadControl::new(),
                    &RecordingProgress(Mutex::new(Vec::new())),
                )
                .await
                .err()
                .unwrap();

            assert_eq!(error.code, AppErrorCode::E009);
        });
    }

    #[test]
    fn acknowledges_pause_before_opening_a_network_request() {
        tauri::async_runtime::block_on(async {
            let (url, requests) = serve(vec![]);
            let (_temp, _output, request) = setup(url);
            let control = DownloadControl::new();
            control.request_pause();

            let result = downloader()
                .download(request, control, &RecordingProgress(Mutex::new(Vec::new())))
                .await
                .unwrap();

            assert_eq!(result, DownloadOutcome::Paused);
            assert!(requests.lock().unwrap().is_empty());
        });
    }

    #[test]
    fn accepts_416_only_when_the_checkpoint_is_already_complete() {
        tauri::async_runtime::block_on(async {
            let (url, _) = serve(vec![
                "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */11\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            ]);
            let (_temp, _output, request) = setup(url);
            fs::write(&request.workspace.source_path, b"hello world").unwrap();
            FsAudioWorkspace
                .save_checkpoint(&audio_paths(&request.workspace), &checkpoint(&request, 11))
                .unwrap();

            let outcome = downloader()
                .download(
                    request,
                    DownloadControl::new(),
                    &RecordingProgress(Mutex::new(Vec::new())),
                )
                .await
                .unwrap();

            assert_eq!(outcome, DownloadOutcome::Completed);
        });
    }

    #[test]
    fn maps_a_short_response_to_retryable_network_failure() {
        tauri::async_runtime::block_on(async {
            let (url, _) = serve(vec![
                "HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nshort",
            ]);
            let (_temp, _output, request) = setup(url);

            let error = downloader()
                .download(
                    request,
                    DownloadControl::new(),
                    &RecordingProgress(Mutex::new(Vec::new())),
                )
                .await
                .err()
                .unwrap();

            assert_eq!(error.code, AppErrorCode::E009);
        });
    }

    #[test]
    fn rejects_crossed_media_kind_and_mime_before_any_request() {
        tauri::async_runtime::block_on(async {
            let (url, requests) = serve(vec![]);
            let (_temp, _output, mut request) = setup(url);
            request.source.mime_type = "video/mp4".into();

            let error = downloader()
                .download(
                    request,
                    DownloadControl::new(),
                    &RecordingProgress(Mutex::new(Vec::new())),
                )
                .await
                .err()
                .unwrap();

            assert_eq!(error.code, AppErrorCode::E004);
            assert!(requests.lock().unwrap().is_empty());
        });
    }

    #[test]
    fn downloads_an_mp4_video_source_through_the_shared_byte_path() {
        tauri::async_runtime::block_on(async {
            let (url, requests) = serve(vec![
                "HTTP/1.1 200 OK\r\nContent-Length: 11\r\nConnection: close\r\n\r\nhello world",
            ]);
            let (_temp, _output, mut request) = setup(url);
            request.source.kind = MediaKind::Video;
            request.source.mime_type = "video/mp4".into();

            let outcome = downloader()
                .download(
                    request.clone(),
                    DownloadControl::new(),
                    &RecordingProgress(Mutex::new(Vec::new())),
                )
                .await
                .unwrap();

            assert_eq!(outcome, DownloadOutcome::Completed);
            assert_eq!(
                fs::read(&request.workspace.source_path).unwrap(),
                b"hello world"
            );
            assert_eq!(requests.lock().unwrap().len(), 1);
        });
    }

    #[test]
    fn cover_download_accepts_images_and_rejects_other_content() {
        tauri::async_runtime::block_on(async {
            let (image_url, _) = serve(vec![
                "HTTP/1.1 200 OK\r\nContent-Type: image/jpeg\r\nContent-Length: 5\r\nConnection: close\r\n\r\ncover",
            ]);
            let image_root = tempfile::tempdir().unwrap();
            let image_path = image_root.path().join("cover.jpg");
            let outcome = downloader()
                .download_cover(&image_url, &image_path, DownloadControl::new())
                .await
                .unwrap();
            assert_eq!(outcome, DownloadOutcome::Completed);
            assert_eq!(fs::read(image_path).unwrap(), b"cover");

            let (text_url, _) = serve(vec![
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 5\r\nConnection: close\r\n\r\nwrong",
            ]);
            let text_root = tempfile::tempdir().unwrap();
            let text_path = text_root.path().join("cover.jpg");
            let error = downloader()
                .download_cover(&text_url, &text_path, DownloadControl::new())
                .await
                .err()
                .unwrap();
            assert_eq!(error.code, AppErrorCode::E008);
            assert!(!text_path.exists());
        });
    }
}
