use super::{AppError, AudioFormat};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioOutputProfile {
    Mp3 { bitrate_kbps: u16 },
    M4aOriginal,
    FlacLossless,
}

impl AudioOutputProfile {
    pub fn try_from_selection(format: AudioFormat, profile_id: &str) -> Result<Self, AppError> {
        match (format, profile_id) {
            (AudioFormat::Mp3, "128") => Ok(Self::Mp3 { bitrate_kbps: 128 }),
            (AudioFormat::Mp3, "192") => Ok(Self::Mp3 { bitrate_kbps: 192 }),
            (AudioFormat::Mp3, "320") => Ok(Self::Mp3 { bitrate_kbps: 320 }),
            (AudioFormat::M4a, "source") => Ok(Self::M4aOriginal),
            (AudioFormat::Flac, "lossless") => Ok(Self::FlacLossless),
            _ => Err(AppError::internal("Invalid audio output profile")),
        }
    }
}
