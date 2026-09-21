//! Transient authored narration and explicit admission as durable source media.
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
    input_text: String,
    /// Defaults are explicit: no cloned voice, instructions or reference audio.
    request: serde_json::Value,
    runtime: RuntimeProvenance,
    output_hash: String,
    duration_millis: u64,
    created_at_millis: i64,
}

/// A validated runtime result held outside the catalog until explicit acceptance.
#[derive(Debug)]
pub struct NarrationCandidate {
    path: PathBuf,
    receipt: Receipt,
}
impl NarrationCandidate {
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
) -> Result<NarrationCandidate, CoreError> {
    let text = text.trim();
    let (bytes, runtime) = crate::infer_runtime::speech::synthesize(config, text)?;
    stage(text, directory, &bytes, runtime)
}

fn stage(
    text: &str,
    directory: &Path,
    bytes: &[u8],
    runtime: RuntimeProvenance,
) -> Result<NarrationCandidate, CoreError> {
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
    Ok(NarrationCandidate {
        path,
        receipt: Receipt {
            schema_version: 1,
            input_text: text.into(),
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
pub fn accept_narration(
    catalog: &Catalog,
    candidate: &NarrationCandidate,
    assembly_id: &str,
    collect_globally: bool,
) -> Result<String, CoreError> {
    let root = catalog
        .path()
        .parent()
        .ok_or_else(|| failure("missing catalog root"))?;
    let receipt = &candidate.receipt;
    let relative = Path::new("media/generated")
        .join(receipt.output_hash.to_string())
        .join("narration.wav");
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
                !independent && (collect_globally || previous.in_materials), "voice")?;
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

fn failure(error: impl std::fmt::Display) -> CoreError {
    CoreError::new(CoreErrorKind::Other, error.to_string())
}

#[cfg(test)]
mod tests;
