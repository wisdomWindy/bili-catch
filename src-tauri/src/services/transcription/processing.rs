use crate::models::TranscriptSegment;

pub fn apply_punctuation_preference(text: &str, punctuation: bool) -> String {
    if punctuation {
        return text.to_owned();
    }
    let characters = text.chars().collect::<Vec<_>>();
    characters
        .iter()
        .enumerate()
        .filter_map(|(index, character)| {
            let previous = index.checked_sub(1).and_then(|value| characters.get(value));
            let next = characters.get(index + 1);
            let preserve_period = *character == '.'
                && previous.is_some_and(|value| value.is_ascii_alphanumeric())
                && next.is_some_and(|value| value.is_ascii_alphanumeric());
            let preserve_path_colon = *character == ':'
                && previous.is_some_and(|value| value.is_ascii_alphabetic())
                && next.is_some_and(|value| *value == '/' || *value == '\\');
            if preserve_period || preserve_path_colon || !is_removable_punctuation(*character) {
                Some(*character)
            } else {
                None
            }
        })
        .collect()
}

pub fn merge_segments(chunks: Vec<Vec<TranscriptSegment>>) -> Vec<TranscriptSegment> {
    let mut merged = Vec::new();
    let mut previous_end = 0;
    for segment in chunks.into_iter().flatten() {
        let text = segment.text.trim();
        if text.is_empty() {
            continue;
        }
        let start_ms = segment.start_ms.max(previous_end);
        let end_ms = segment.end_ms.max(start_ms);
        previous_end = end_ms;
        merged.push(TranscriptSegment {
            index: merged.len() as u32,
            start_ms,
            end_ms,
            text: text.into(),
        });
    }
    merged
}

fn is_removable_punctuation(character: char) -> bool {
    matches!(
        character,
        ',' | '.'
            | '!'
            | '?'
            | ';'
            | ':'
            | '，'
            | '。'
            | '！'
            | '？'
            | '；'
            | '：'
            | '、'
            | '“'
            | '”'
            | '‘'
            | '’'
    )
}
