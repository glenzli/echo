//! Local speech synthesis with the same authenticated SDK and evidence checks
//! as editor analysis. Text is authored narration, never transcript evidence.
use super::{InferRuntimeConfig, InferRuntimeError, RuntimeProvenance};
use infer_runtime_client::{ExecutionMode, SpeechFormat, SpeechRequest};

pub(crate) fn synthesize(
    config: InferRuntimeConfig,
    text: &str,
) -> Result<(Vec<u8>, RuntimeProvenance), InferRuntimeError> {
    validate_text(text)?;
    let client = super::build_sdk_client(config)?;
    let request = request(text);
    let (response, snapshot) = super::SdkTransport::run(async {
        let response = client.synthesize_speech(&request).await?;
        let snapshot = client.job(&response.job_id).await?;
        Ok((response, snapshot))
    })?;
    if response.logical_model != request.model
        || response.job_id != snapshot.id
        || !matches!(
            response.content_type.split(';').next(),
            Some("audio/wav" | "audio/x-wav")
        )
        || response.bytes.is_empty()
        || response.bytes.len() > 32 * 1024 * 1024
    {
        return Err(super::protocol("inconsistent_speech_response"));
    }
    let job = super::validate_succeeded_job(
        snapshot,
        "speech.synthesize",
        "infer.audio.speech@20260811.1",
    )?;
    super::validate_local_only_job(&job, "inconsistent_speech_constraints")?;
    if job.model_build.is_empty() || job.physical_model.is_empty() || job.id.is_empty() {
        return Err(super::protocol("missing_speech_identity"));
    }
    Ok((
        response.bytes,
        RuntimeProvenance {
            contract_version: super::EXPECTED_CONTRACT_VERSION.into(),
            job,
        },
    ))
}

/// Versioned synthetic preset resolved by Runtime, never a cloned voice.
pub(crate) fn request(text: &str) -> SpeechRequest {
    SpeechRequest {
        model: "speech.synthesize".into(),
        input: text.to_owned(),
        voice: Some("speech.voice.zh.bright_female.v1".into()),
        instructions: None,
        language: None,
        speed: 1.0,
        response_format: SpeechFormat::Wav,
        execution_mode: ExecutionMode::Unary,
        metadata: super::default_background_constraints(),
    }
}

pub(crate) fn validate_text(text: &str) -> Result<(), InferRuntimeError> {
    if text.trim().is_empty() || text.chars().count() > 500 || text.contains('\0') {
        return Err(super::rejected("narration_requires_1_to_500_characters"));
    }
    Ok(())
}
