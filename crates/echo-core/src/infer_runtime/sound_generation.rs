//! Bounded local sound materials through the authenticated, capability-negotiated SDK.
use super::{InferRuntimeConfig, InferRuntimeError, RuntimeProvenance};
use crate::{SoundMaterialModel, SoundMaterialSpec};
use infer_runtime_client::{SoundGenerationRequest, SoundModelChoice};

pub(crate) fn request(
    spec: &SoundMaterialSpec,
) -> Result<SoundGenerationRequest, InferRuntimeError> {
    spec.validate()
        .map_err(|_| super::rejected("invalid_sound_effect_request"))?;
    Ok(SoundGenerationRequest {
        model: "audio.generate_sound".into(),
        model_choice: Some(match spec.model {
            SoundMaterialModel::SmallSfx => SoundModelChoice::SmallSfx,
            SoundMaterialModel::SmallMusic => SoundModelChoice::SmallMusic,
            SoundMaterialModel::OpenSmall => SoundModelChoice::OpenSmall,
        }),
        prompt: spec.normalized_prompt(),
        duration_seconds: u8::try_from(spec.duration_seconds)
            .map_err(|_| super::rejected("invalid_sound_effect_duration"))?,
        seed: Some(spec.seed),
        metadata: super::default_background_constraints(),
    })
}

pub(crate) fn generate(
    config: InferRuntimeConfig,
    spec: &SoundMaterialSpec,
) -> Result<(Vec<u8>, RuntimeProvenance, serde_json::Value), InferRuntimeError> {
    let request = request(spec)?;
    let client = super::build_sdk_client(config)?;
    let (response, snapshot) = super::SdkTransport::run(async {
        let response = client.generate_sound_effect(&request).await?;
        let snapshot = client.job(&response.job_id).await?;
        Ok((response, snapshot))
    })?;
    // The SDK checks the WAV envelope and SHA-256. Bind its transport identity
    // to the successful local Job before creating a reviewable candidate.
    if response.logical_model != request.model
        || Some(response.model_choice) != request.model_choice
        || response.job_id != snapshot.id
        || response.provider != snapshot.provider
        || response.deployment != snapshot.deployment
        || response.model_build != snapshot.model_build
        || response.physical_model != snapshot.physical_model
        || response.placement != "local"
        || response.seed != spec.seed
        || response.duration_seconds != request.duration_seconds
        || response.wav.is_empty()
        || response.wav.len() > 16 * 1024 * 1024
    {
        return Err(super::protocol("inconsistent_sound_generation_response"));
    }
    let job = super::validate_succeeded_job(
        snapshot,
        "audio.generate_sound",
        "infer.audio.sound-generation@20260926.2",
    )?;
    super::validate_local_only_job(&job, "inconsistent_sound_generation_constraints")?;
    if job.model_build.is_empty() || job.physical_model.is_empty() || job.id.is_empty() {
        return Err(super::protocol("missing_sound_generation_identity"));
    }
    let request = serde_json::to_value(request)
        .map_err(|_| super::protocol("invalid_sound_generation_request"))?;
    Ok((
        response.wav,
        RuntimeProvenance {
            contract_version: super::EXPECTED_CONTRACT_VERSION.into(),
            job,
        },
        request,
    ))
}
