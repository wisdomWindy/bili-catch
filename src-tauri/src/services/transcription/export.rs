use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;

use crate::models::{AppError, AppErrorCode, TranscriptFormat, TranscriptSegment};

pub fn render_transcript(
    segments: &[TranscriptSegment],
    language: &str,
    format: TranscriptFormat,
    timestamps: bool,
) -> Result<String, AppError> {
    match format {
        TranscriptFormat::Txt => Ok(render_txt(segments)),
        TranscriptFormat::Srt => Ok(render_srt(segments)),
        TranscriptFormat::Vtt => Ok(render_vtt(segments)),
        TranscriptFormat::Json => render_json(segments, language),
        TranscriptFormat::Markdown => Ok(render_markdown(segments, timestamps)),
    }
}

pub fn choose_export_path(
    directory: &Path,
    source_name: &str,
    language: &str,
    format: TranscriptFormat,
) -> Result<PathBuf, AppError> {
    let source_stem = Path::new(source_name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("transcript");
    let mut stem = sanitize_component(source_stem);
    if stem.is_empty() {
        stem = "transcript".into();
    }
    let language = sanitize_component(language);
    let language = if language.is_empty() {
        "und"
    } else {
        &language
    };
    let extension = format_extension(format);
    let reserved = language.chars().count() + extension.len() + 2;
    stem = stem
        .chars()
        .take(200usize.saturating_sub(reserved))
        .collect();

    for suffix in 0..10_000 {
        let counter = if suffix == 0 {
            String::new()
        } else {
            format!(" ({suffix})")
        };
        let counter_length = counter.chars().count();
        let bounded_stem = stem
            .chars()
            .take(200usize.saturating_sub(reserved + counter_length))
            .collect::<String>();
        let candidate = directory.join(format!("{bounded_stem}.{language}{counter}.{extension}"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(export_error(
        "EXPORT_NAME_UNAVAILABLE",
        "A transcript output name could not be reserved",
    ))
}

pub fn write_export_atomic(path: &Path, content: &str) -> Result<(), AppError> {
    let parent = path.parent().ok_or_else(|| {
        export_error(
            "EXPORT_DIRECTORY_INVALID",
            "The transcript directory is invalid",
        )
    })?;
    fs::create_dir_all(parent).map_err(|_| {
        export_error(
            "EXPORT_DIRECTORY_UNAVAILABLE",
            "The transcript directory is unavailable",
        )
    })?;
    let mut temporary_name = path.as_os_str().to_owned();
    temporary_name.push(".part");
    let temporary = PathBuf::from(temporary_name);
    let result = (|| {
        fs::write(&temporary, content.as_bytes()).map_err(|_| {
            export_error("EXPORT_WRITE_FAILED", "The transcript could not be written")
        })?;
        fs::rename(&temporary, path).map_err(|_| {
            export_error(
                "EXPORT_RENAME_FAILED",
                "The transcript could not be published",
            )
        })
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

fn render_txt(segments: &[TranscriptSegment]) -> String {
    let mut output = segments
        .iter()
        .filter_map(|segment| {
            let text = segment.text.trim();
            (!text.is_empty()).then_some(text)
        })
        .collect::<Vec<_>>()
        .join("\n");
    output.push('\n');
    output
}

fn render_srt(segments: &[TranscriptSegment]) -> String {
    segments
        .iter()
        .enumerate()
        .map(|(index, segment)| {
            format!(
                "{}\n{} --> {}\n{}\n",
                index + 1,
                timestamp(segment.start_ms, ','),
                timestamp(segment.end_ms.max(segment.start_ms), ','),
                segment.text.trim()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_vtt(segments: &[TranscriptSegment]) -> String {
    let body = segments
        .iter()
        .map(|segment| {
            format!(
                "{} --> {}\n{}\n",
                timestamp(segment.start_ms, '.'),
                timestamp(segment.end_ms.max(segment.start_ms), '.'),
                segment.text.trim()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("WEBVTT\n\n{body}")
}

fn render_json(segments: &[TranscriptSegment], language: &str) -> Result<String, AppError> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct JsonTranscript<'a> {
        language: &'a str,
        segments: &'a [TranscriptSegment],
    }
    let mut output = serde_json::to_string_pretty(&JsonTranscript { language, segments })
        .map_err(|_| export_error("EXPORT_SERIALIZE_FAILED", "The transcript is invalid"))?;
    output.push('\n');
    Ok(output)
}

fn render_markdown(segments: &[TranscriptSegment], timestamps: bool) -> String {
    let mut output = segments
        .iter()
        .map(|segment| {
            if timestamps {
                format!(
                    "- [{}] {}",
                    timestamp(segment.start_ms, '.'),
                    segment.text.trim()
                )
            } else {
                segment.text.trim().to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    output.push('\n');
    output
}

fn timestamp(milliseconds: u64, separator: char) -> String {
    let hours = milliseconds / 3_600_000;
    let minutes = milliseconds / 60_000 % 60;
    let seconds = milliseconds / 1_000 % 60;
    let millis = milliseconds % 1_000;
    format!("{hours:02}:{minutes:02}:{seconds:02}{separator}{millis:03}")
}

fn sanitize_component(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(|character| {
            if matches!(
                character,
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
            ) || character.is_control()
            {
                '_'
            } else {
                character
            }
        })
        .collect::<String>()
        .trim_end_matches([' ', '.'])
        .to_owned()
}

fn format_extension(format: TranscriptFormat) -> &'static str {
    match format {
        TranscriptFormat::Txt => "txt",
        TranscriptFormat::Srt => "srt",
        TranscriptFormat::Vtt => "vtt",
        TranscriptFormat::Json => "json",
        TranscriptFormat::Markdown => "md",
    }
}

fn export_error(details: &str, message: &str) -> AppError {
    AppError {
        code: AppErrorCode::E016,
        message: message.into(),
        details: Some(details.into()),
    }
}
