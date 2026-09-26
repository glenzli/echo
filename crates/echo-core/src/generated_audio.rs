//! Generated audio candidates and explicit admission as durable source media.
//! Candidate handles cannot be constructed from UI JSON. Catalog visibility,
//! membership and immutable generation evidence publish in one transaction.
use crate::{CoreError, CoreErrorKind, InferRuntimeConfig, RuntimeProvenance};
use echo_catalog::{AssetRegistrationInput, Catalog, RegisterAsset};
use echo_domain::ContentHash;
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Serialize)]
struct Receipt {
    schema_version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    material_category: Option<String>,
    input_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt_preparation: Option<infer_runtime_client::PreparedSoundPrompt>,
    /// The exact bounded request sent to Runtime; source bytes stay independent.
    request: serde_json::Value,
    runtime: RuntimeProvenance,
    output_hash: String,
    duration_millis: u64,
    created_at_millis: i64,
}

/// A validated runtime result held outside the catalog until explicit acceptance.
#[derive(Debug)]
pub struct GeneratedAudioCandidate {
    path: PathBuf,
    receipt: Receipt,
}
impl GeneratedAudioCandidate {
    /// Review-only projection. Acceptance uses the opaque handle, never this JSON.
    /// # Errors
    /// Returns serialization errors.
    pub fn details_json(&self) -> Result<String, CoreError> {
        serde_json::to_string(&self.receipt).map_err(failure)
    }
}

/// Generates local narration into a caller-owned temporary directory.
/// # Errors
/// Rejects invalid requests, unprovenanced results and invalid/oversized audio.
pub fn generate_narration(
    text: &str,
    directory: &Path,
    config: InferRuntimeConfig,
) -> Result<GeneratedAudioCandidate, CoreError> {
    let text = text.trim();
    let (bytes, runtime) = crate::infer_runtime::speech::synthesize(config, text)?;
    stage(text, directory, &bytes, runtime)
}

fn stage(
    text: &str,
    directory: &Path,
    bytes: &[u8],
    runtime: RuntimeProvenance,
) -> Result<GeneratedAudioCandidate, CoreError> {
    use std::io::Write;
    crate::infer_runtime::speech::validate_text(text)?;
    if bytes.is_empty() || bytes.len() > 32 * 1024 * 1024 {
        return Err(failure("invalid narration size"));
    }
    let path = directory.join("narration.wav");
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(failure)?;
    file.write_all(bytes).map_err(failure)?;
    file.sync_all().map_err(failure)?;
    let probe = echo_bridge::probe(&path).map_err(|e| failure(e.message))?;
    if !probe.has_audio || probe.duration_millis == 0 || probe.duration_millis > 120_000 {
        return Err(failure("narration must be between zero and two minutes"));
    }
    Ok(GeneratedAudioCandidate {
        path,
        receipt: Receipt {
            schema_version: 1,
            generation_kind: None,
            material_category: None,
            input_text: text.into(),
            prompt_preparation: None,
            request: serde_json::to_value(crate::infer_runtime::speech::request(text))
                .map_err(failure)?,
            runtime,
            output_hash: ContentHash::from(blake3::hash(bytes)).to_string(),
            duration_millis: probe.duration_millis,
            created_at_millis: crate::util::now_millis(),
        },
    })
}

/// Accepts exactly the auditioned bytes, with mandatory source disclosure.
/// # Errors
/// Rejects tampered candidates/destinations and invalid project destinations.
pub fn accept_generated_audio(
    catalog: &Catalog,
    candidate: &GeneratedAudioCandidate,
    assembly_id: &str,
    collect_globally: bool,
) -> Result<String, CoreError> {
    let root = catalog
        .path()
        .parent()
        .ok_or_else(|| failure("missing catalog root"))?;
    let receipt = &candidate.receipt;
    if receipt.schema_version == 3 {
        receipt
            .prompt_preparation
            .as_ref()
            .ok_or_else(|| failure("missing sound prompt preparation"))?
            .validate_for_generation(&receipt.input_text, "echo")
            .map_err(failure)?;
    }
    let relative = Path::new("media/generated")
        .join(&receipt.output_hash)
        .join(if receipt.generation_kind.is_some() {
            "sound.wav"
        } else {
            "narration.wav"
        });
    let destination = root.join(&relative);
    let directory = destination
        .parent()
        .ok_or_else(|| failure("missing media root"))?;
    fs::create_dir_all(directory).map_err(failure)?;
    let staging = directory.join(format!(".{}.part", echo_domain::AssetId::new()));
    let result = (|| {
        fs::copy(&candidate.path, &staging).map_err(failure)?;
        if crate::hash_file(&staging)?.to_string() != receipt.output_hash {
            return Err(failure("candidate audio changed; generate again"));
        }
        fs::File::open(&staging)
            .and_then(|f| f.sync_all())
            .map_err(failure)?;
        let probe = echo_bridge::probe(&staging).map_err(|e| failure(e.message))?;
        let json = serde_json::to_value(receipt).map_err(failure)?;
        catalog.with_transaction(|tx| -> Result<String, CoreError> {
            let independent: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM catalog_meta WHERE key='session_kind' AND value='independent-editor-v1')", [], |r| r.get(0)).map_err(echo_catalog::CatalogError::from)?;
            if !independent && !collect_globally && assembly_id.is_empty() {
                return Err(failure("choose a project or collect the material"));
            }
            tx.execute("UPDATE catalog_meta SET value=value WHERE key='schema_version'", []).map_err(echo_catalog::CatalogError::from)?;
            // Hold Catalog's writer transaction while installing and publishing bytes.
            // Failed admission removes only the file this transaction installed.
            let installed = match fs::hard_link(&staging, &destination) {
                Ok(()) => true,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    if crate::hash_file(&destination)?.to_string() != receipt.output_hash {
                        return Err(failure("generated source integrity check failed"));
                    }
                    false
                },
                Err(e) => return Err(failure(e)),
            };
            let admitted = (|| -> Result<String, CoreError> {
            let (asset, created) = match echo_catalog::register_asset(tx, &AssetRegistrationInput {
                content_hash: receipt.output_hash.parse().map_err(failure)?,
                path: if independent { &relative } else { &destination },
                size_bytes: destination.metadata().map_err(failure)?.len(),
                codec: Some(&probe.codec_name), duration_millis: Some(probe.duration_millis),
                recorded_at_millis: None, imported_at_millis: crate::util::now_millis(),
            })? {
                RegisterAsset::Created(a) => (a, true),
                RegisterAsset::Existed(a) => (a, false),
            };
            let memberships = echo_catalog::sound_memberships(tx)?;
            let previous = &memberships[&asset.id.to_string()];
            echo_catalog::set_sound_membership(tx, &asset.id.to_string(), !created && previous.in_memory,
                !independent && (collect_globally || previous.in_materials), receipt.material_category.as_deref().unwrap_or("voice"))?;
            if !assembly_id.is_empty() {
                echo_catalog::attach_project_material(tx, assembly_id, &asset.id.to_string())?;
            }
            if created { echo_catalog::record_source_metadata(tx, asset.id, &echo_catalog::SourceMetadata {
                container_format: probe.container_format,
                sample_rate: probe.sample_rate, channel_count: probe.channel_count,
                entries: vec![echo_catalog::SourceMetadataEntry { key: "title".into(), value: receipt.input_text.chars().take(60).collect() }],
            }, None)?; }
            echo_catalog::record_generated_audio(tx, asset.id, &json)?;
            Ok(asset.id.to_string())
            })();
            if admitted.is_err() && installed { let _ = fs::remove_file(&destination); }
            admitted
        })
    })();
    let _ = fs::remove_file(&staging);
    result
}

/// Explicit local model choice; routing and deployment remain Runtime-owned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundMaterialModel {
    SmallSfx,
    SmallMusic,
    OpenSmall,
}

impl SoundMaterialModel {
    /// Stable user-facing model choice; does not name a deployment or local path.
    pub fn choice(self) -> &'static str {
        match self {
            Self::SmallSfx => "stable_audio_3_small_sfx",
            Self::SmallMusic => "stable_audio_3_small_music",
            Self::OpenSmall => "stable_audio_open_small",
        }
    }

    /// Maximum duration admitted by Echo for this short-material workflow.
    pub fn max_seconds(self) -> u32 {
        if self == Self::OpenSmall { 11 } else { 30 }
    }

    /// Decodes an explicit model choice from the editor.
    /// # Errors
    /// Rejects unsupported model identities instead of silently substituting one.
    pub fn from_choice(name: &str) -> Result<Self, CoreError> {
        match name {
            "stable_audio_3_small_sfx" => Ok(Self::SmallSfx),
            "stable_audio_3_small_music" => Ok(Self::SmallMusic),
            "stable_audio_open_small" => Ok(Self::OpenSmall),
            _ => Err(failure("unsupported sound material model")),
        }
    }
}

/// Short sound generation inputs. Seeds are retained for repeatable requests.
#[derive(Debug, Clone)]
pub struct SoundMaterialSpec {
    pub model: SoundMaterialModel,
    pub prompt: String,
    pub duration_seconds: u32,
    pub seed: u32,
    pub ambience: bool,
}

impl SoundMaterialSpec {
    pub(crate) fn normalized_prompt(&self) -> String {
        self.prompt.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// Checks the editor's short-material limits before contacting Runtime.
    /// # Errors
    /// Rejects empty/oversized prompts and durations outside 1–30 seconds.
    pub fn validate(&self) -> Result<(), CoreError> {
        if self.prompt.trim().is_empty()
            || self.prompt.chars().count() > 500
            || self
                .prompt
                .chars()
                .any(|c| c.is_control() && !c.is_whitespace())
            || !(1..=self.model.max_seconds()).contains(&self.duration_seconds)
        {
            return Err(failure(
                "sound effects require 1–500 prompt characters and 1–30 seconds",
            ));
        }
        Ok(())
    }
}

/// Generates a sound-effect candidate outside the catalog; acceptance is separate.
/// # Errors
/// Rejects invalid input, runtime evidence, and mismatched or oversized audio.
pub fn generate_sound_material(
    spec: &SoundMaterialSpec,
    directory: &Path,
    config: InferRuntimeConfig,
) -> Result<GeneratedAudioCandidate, CoreError> {
    spec.validate()?;
    let preparation = prepare_sound_material_prompt(&spec.prompt, config.clone())?;
    generate_prepared_sound_material(spec, &preparation, directory, config)
}

/// Prepare once, then retain this validated provenance for candidate retries.
/// # Errors
/// Rejects invalid prompts and unavailable or inconsistent local text Jobs.
pub fn prepare_sound_material_prompt(
    prompt: &str,
    config: InferRuntimeConfig,
) -> Result<String, CoreError> {
    let normalized = prompt.split_whitespace().collect::<Vec<_>>().join(" ");
    if prompt.chars().count() > 500 {
        return Err(failure("prompt exceeds 500 characters"));
    }
    let prepared = crate::infer_runtime::sound_generation::prepare(config, &normalized)?;
    serde_json::to_string(&prepared).map_err(failure)
}

/// Generate from a previously prepared prompt without repeating the text Job.
/// # Errors
/// Rejects stale/invalid preparation, invalid audio and inconsistent provenance.
pub fn generate_prepared_sound_material(
    spec: &SoundMaterialSpec,
    preparation: &str,
    directory: &Path,
    config: InferRuntimeConfig,
) -> Result<GeneratedAudioCandidate, CoreError> {
    spec.validate()?;
    let prepared = crate::infer_runtime::sound_generation::decode_preparation(
        preparation,
        &spec.normalized_prompt(),
    )?;
    let (bytes, runtime, request) =
        crate::infer_runtime::sound_generation::generate(config, spec, &prepared)?;
    let mut candidate = stage_sound_material(spec, directory, &bytes, runtime, request)?;
    candidate.receipt.schema_version = 3;
    candidate.receipt.prompt_preparation = Some(prepared);
    Ok(candidate)
}

fn stage_sound_material(
    spec: &SoundMaterialSpec,
    directory: &Path,
    bytes: &[u8],
    runtime: RuntimeProvenance,
    request: serde_json::Value,
) -> Result<GeneratedAudioCandidate, CoreError> {
    use std::io::Write;
    spec.validate()?;
    if bytes.is_empty() || bytes.len() > 16 * 1024 * 1024 {
        return Err(failure("invalid sound-effect size"));
    }
    let path = directory.join("sound.wav");
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(failure)?;
    file.write_all(bytes).map_err(failure)?;
    file.sync_all().map_err(failure)?;
    let probe = echo_bridge::probe(&path).map_err(|e| failure(e.message))?;
    if !probe.has_audio
        || probe
            .duration_millis
            .abs_diff(u64::from(spec.duration_seconds) * 1000)
            > 100
        || probe.channel_count == 0
        || probe.channel_count > 2
    {
        return Err(failure(
            "sound-effect audio does not match the requested duration or channels",
        ));
    }
    Ok(GeneratedAudioCandidate {
        path,
        receipt: Receipt {
            schema_version: 2,
            generation_kind: Some(
                if spec.model == SoundMaterialModel::SmallMusic {
                    "music"
                } else {
                    "sound_effect"
                }
                .into(),
            ),
            material_category: Some(
                if spec.model == SoundMaterialModel::SmallMusic {
                    "music"
                } else if spec.ambience {
                    "ambience"
                } else {
                    "effects"
                }
                .into(),
            ),
            input_text: spec.normalized_prompt(),
            prompt_preparation: None,
            request,
            runtime,
            output_hash: ContentHash::from(blake3::hash(bytes)).to_string(),
            duration_millis: probe.duration_millis,
            created_at_millis: crate::util::now_millis(),
        },
    })
}

fn failure(error: impl std::fmt::Display) -> CoreError {
    CoreError::new(CoreErrorKind::Other, error.to_string())
}

#[cfg(test)]
mod tests;
