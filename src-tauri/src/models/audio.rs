use super::{AppError, AudioFormat};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioOutputProfile {
    Mp3 { bitrate_kbps: u16 },
    M4aOriginal,
    FlacLossless,
    /// PCM WAV：有损源按 16 bit 解，无损源按 24 bit 解，采样率保持不变。
    WavPcm,
}

impl AudioOutputProfile {
    pub fn try_from_selection(format: AudioFormat, profile_id: &str) -> Result<Self, AppError> {
        match (format, profile_id) {
            (AudioFormat::Mp3, "128") => Ok(Self::Mp3 { bitrate_kbps: 128 }),
            (AudioFormat::Mp3, "192") => Ok(Self::Mp3 { bitrate_kbps: 192 }),
            (AudioFormat::Mp3, "320") => Ok(Self::Mp3 { bitrate_kbps: 320 }),
            (AudioFormat::M4a, "source") => Ok(Self::M4aOriginal),
            (AudioFormat::Flac, "lossless") => Ok(Self::FlacLossless),
            (AudioFormat::Wav, "source") => Ok(Self::WavPcm),
            _ => Err(AppError::internal("Invalid audio output profile")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AudioOutputProfile;
    use crate::models::AudioFormat;

    #[test]
    fn wav_uses_the_single_source_profile() {
        assert_eq!(
            AudioOutputProfile::try_from_selection(AudioFormat::Wav, "source").unwrap(),
            AudioOutputProfile::WavPcm
        );
        // WAV 没有码率档位，128 之类的 MP3 档位必须被拒绝
        assert!(AudioOutputProfile::try_from_selection(AudioFormat::Wav, "320").is_err());
    }
}
