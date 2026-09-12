const MAX_FILE_NAME_CHARS: usize = 200;

pub(super) fn audio_extension(format: crate::models::AudioFormat) -> &'static str {
    match format {
        crate::models::AudioFormat::Mp3 => "mp3",
        crate::models::AudioFormat::M4a => "m4a",
        crate::models::AudioFormat::Flac => "flac",
    }
}

pub fn sanitize_filename(part_title: &str, bvid: &str, page: u32, extension: &str) -> String {
    finish_filename(normalize_stem(part_title), bvid, page, extension)
}

pub fn sanitize_audio_filename(
    video_title: &str,
    part_title: &str,
    bvid: &str,
    page: u32,
    part_count: u32,
    extension: &str,
) -> String {
    let stem = if part_count == 1 {
        normalize_stem(video_title)
    } else {
        let part = normalize_stem(part_title);
        (!part.is_empty())
            .then(|| format!("{page}_{part}"))
            .unwrap_or_default()
    };
    finish_filename(stem, bvid, page, extension)
}

fn normalize_stem(value: &str) -> String {
    let filtered: String = value
        .chars()
        .map(|character| {
            if character.is_control() || character.is_whitespace() {
                ' '
            } else {
                character
            }
        })
        .filter(|character| {
            !matches!(
                character,
                '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|'
            )
        })
        .collect();
    filtered
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches('.')
        .to_owned()
}

fn finish_filename(mut stem: String, bvid: &str, page: u32, extension: &str) -> String {
    if stem.is_empty() {
        stem = format!("{bvid}-P{page}");
    }
    if is_windows_reserved(&stem) {
        stem.push('_');
    }

    let extension = extension.trim_start_matches('.');
    let suffix = format!(".{extension}");
    let max_stem_chars = MAX_FILE_NAME_CHARS.saturating_sub(suffix.chars().count());
    stem = stem.chars().take(max_stem_chars).collect();
    format!("{stem}{suffix}")
}

fn is_windows_reserved(stem: &str) -> bool {
    let base = stem.split('.').next().unwrap_or(stem).to_ascii_uppercase();
    matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (base.len() == 4
            && (base.starts_with("COM") || base.starts_with("LPT"))
            && matches!(base.as_bytes()[3], b'1'..=b'9'))
}
