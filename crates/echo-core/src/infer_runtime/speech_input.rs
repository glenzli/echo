//! Temporary speech-model input compatibility; original audio stays immutable.
//! The local speech worker accepts a narrower container family than Echo's decoder.
use super::{InferRuntimeError, protocol, rejected};
use std::path::{Path, PathBuf};

pub(super) struct SpeechInput {
    path: PathBuf,
    directory: Option<PathBuf>,
}
impl SpeechInput {
    pub(super) fn prepare(source: &Path) -> Result<Self, InferRuntimeError> {
        let extension = source
            .extension()
            .and_then(|value| value.to_str())
            .map(str::to_ascii_lowercase);
        if matches!(
            extension.as_deref(),
            Some("wav" | "mp3" | "flac" | "ogg" | "opus" | "m4a")
        ) {
            return Ok(Self {
                path: source.to_owned(),
                directory: None,
            });
        }
        let probe =
            echo_bridge::probe(source).map_err(|_| rejected("speech_source_undecodable"))?;
        if !probe.has_audio || probe.duration_millis == 0 || probe.duration_millis > 600_000 {
            return Err(rejected("speech_input_requires_bounded_audio"));
        }
        let before = crate::hash_file(source).map_err(|_| rejected("speech_source_unavailable"))?;
        let directory =
            std::env::temp_dir().join(format!("echo-speech-input-{}", echo_domain::AssetId::new()));
        std::fs::create_dir(&directory).map_err(|_| protocol("speech_proxy_unavailable"))?;
        let input = Self {
            path: directory.join("input.wav"),
            directory: Some(directory),
        };
        echo_bridge::build_analysis_proxy(source, input.path(), 0, probe.duration_millis)
            .map_err(|_| rejected("speech_source_undecodable"))?;
        if crate::hash_file(source).map_err(|_| rejected("speech_source_unavailable"))? != before {
            return Err(rejected("speech_source_changed"));
        }
        super::validate_source(input.path())?;
        Ok(input)
    }
    pub(super) fn path(&self) -> &Path {
        &self.path
    }
}
impl Drop for SpeechInput {
    fn drop(&mut self) {
        if let Some(directory) = &self.directory {
            let _ = std::fs::remove_dir_all(directory);
        }
    }
}

#[cfg(test)]
mod tests;
