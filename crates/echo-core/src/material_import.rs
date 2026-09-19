//! Durable, model-independent material intake for global and project bins.
//! Copies are source media beside the catalog, never rebuildable cache entries.

use std::{fs, path::Path};

use crate::{CoreError, CoreErrorKind};
use echo_catalog::{
    AssetRegistrationInput, Catalog, JobKind, MaterialImportPayload, RegisterAsset,
};

pub(crate) fn import_material(
    catalog: &Catalog,
    request: &MaterialImportPayload,
) -> Result<(), CoreError> {
    request.validate()?;
    let root = catalog
        .path()
        .parent()
        .ok_or_else(|| failure("catalog has no media directory"))?
        .join("media/materials");
    fs::create_dir_all(&root).map_err(io_error)?;
    // A unique staging file makes interrupted and concurrent imports harmless.
    let staging = root.join(format!(".{}.part", echo_domain::AssetId::new()));
    let result = import_staged(catalog, request, &root, &staging);
    let _ = fs::remove_file(&staging);
    result
}

fn import_staged(
    catalog: &Catalog,
    request: &MaterialImportPayload,
    root: &Path,
    staging: &Path,
) -> Result<(), CoreError> {
    fs::copy(&request.path, staging).map_err(io_error)?;
    let hash = crate::hash_file(staging)?;
    let probe = echo_bridge::probe(staging).map_err(|e| failure(&e.message))?;
    if !probe.has_audio || probe.duration_millis == 0 {
        return Err(failure("material has no decodable audio"));
    }
    let size = fs::metadata(staging).map_err(io_error)?.len();
    let directory = root.join(hash.to_string());
    fs::create_dir_all(&directory).map_err(io_error)?;
    let destination = directory.join(
        request
            .path
            .file_name()
            .ok_or_else(|| failure("material has no filename"))?,
    );
    match fs::hard_link(staging, &destination) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if crate::hash_file(&destination)? != hash {
                return Err(failure(
                    "managed material content does not match its identity",
                ));
            }
        }
        Err(error) => return Err(io_error(error)),
    }
    // Flush owned bytes before a transaction can publish a durable reference.
    fs::File::open(&destination)
        .and_then(|file| file.sync_all())
        .map_err(io_error)?;
    let now = crate::util::now_millis();
    catalog.with_transaction(|tx| -> Result<(), CoreError> {
        let registration = echo_catalog::register_asset(
            tx,
            &AssetRegistrationInput {
                content_hash: hash,
                path: &destination,
                size_bytes: size,
                codec: Some(&probe.codec_name),
                duration_millis: Some(probe.duration_millis),
                recorded_at_millis: (probe.recorded_at_millis > 0)
                    .then_some(probe.recorded_at_millis),
                imported_at_millis: now,
            },
        )?;
        let (asset, created) = match registration {
            RegisterAsset::Created(a) => (a, true),
            RegisterAsset::Existed(a) => (a, false),
        };
        let memberships = echo_catalog::sound_memberships(tx)?;
        let previous = &memberships[&asset.id.to_string()];
        echo_catalog::set_sound_membership(
            tx,
            &asset.id.to_string(),
            !created && previous.in_memory,
            request.collect_globally || previous.in_materials,
            if request.category.is_empty() {
                &previous.material_category
            } else {
                &request.category
            },
        )?;
        if !request.assembly_id.is_empty() {
            echo_catalog::attach_project_material(tx, &request.assembly_id, &asset.id.to_string())?;
        }
        echo_catalog::record_source_metadata(
            tx,
            asset.id,
            &echo_catalog::SourceMetadata {
                container_format: probe.container_format.clone(),
                sample_rate: probe.sample_rate,
                channel_count: probe.channel_count,
                entries: probe
                    .metadata
                    .iter()
                    .map(|e| echo_catalog::SourceMetadataEntry {
                        key: e.key.clone(),
                        value: e.value.clone(),
                    })
                    .collect(),
            },
            (probe.recorded_at_millis > 0).then_some(probe.recorded_at_millis),
        )?;
        echo_catalog::enqueue_job(
            tx,
            &format!("waveform-{}", asset.id),
            JobKind::AnalyzeWaveform,
            &serde_json::json!({"asset_id":asset.id.to_string()}),
            now,
        )?;
        crate::analysis_queue::enqueue_transcription(tx, asset.id, now)?;
        Ok(())
    })
}

fn failure(message: &str) -> CoreError {
    CoreError::new(CoreErrorKind::SourceUnavailable, message)
}
#[allow(clippy::needless_pass_by_value)] // Direct map_err adapter consumes the I/O error.
fn io_error(error: std::io::Error) -> CoreError {
    failure(&format!("cannot preserve material: {error}"))
}

#[cfg(test)]
mod tests;
