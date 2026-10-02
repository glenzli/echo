//! Validation and durable publication handoff for desktop offline renders.
//! File hashing stays off the Qt thread and Catalog remains the sole owner of
//! provenance persistence.

use std::path::{Path, PathBuf};

use echo_catalog::{
    AssetLookup, RecordRenderExport, RenderExportFormat, find_by_id,
    list_rendered_spectral_working_copies, record_render_export,
    record_rendered_spectral_working_copy_export,
};
use echo_domain::AssetId;

use crate::session::{LibrarySession, SessionError, now_millis};

// Resolve the existing target (including symlinks), or its real parent
// when the destination has not yet been created. Never open it for writing.
fn export_path_identity(path: &Path) -> std::io::Result<PathBuf> {
    match path.canonicalize() {
        Ok(path) => Ok(path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            // Do not turn a dangling symlink into an apparently new target.
            if std::fs::symlink_metadata(path).is_ok_and(|entry| entry.file_type().is_symlink()) {
                return Err(error);
            }
            let absolute = std::path::absolute(path)?;
            let parent = absolute.parent().ok_or(error)?;
            Ok(parent.canonicalize()?.join(
                absolute
                    .file_name()
                    .ok_or_else(|| std::io::Error::other("export destination has no file name"))?,
            ))
        }
        Err(error) => Err(error),
    }
}

impl LibrarySession {
    /// Rejects a delivery that would replace any registered Original, including
    /// project-only materials and sources outside the current mix. Call before
    /// rendering and immediately before atomic publication; a post-write
    /// provenance check cannot protect the source bytes.
    pub(crate) fn validate_export_destination(
        &self,
        output_path: &str,
    ) -> Result<(), SessionError> {
        let destination =
            export_path_identity(Path::new(output_path)).map_err(|error| SessionError {
                message: format!("cannot resolve export destination: {error}"),
            })?;
        let paths = self.catalog().with_transaction(|transaction| {
            let mut statement = transaction.prepare("SELECT path FROM assets")?;
            statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(echo_catalog::CatalogError::from)
        })?;
        // File-system inspection must not hold the Catalog's writer mutex.
        #[cfg(unix)]
        let destination_metadata = destination.metadata().ok();
        for path in paths {
            let same_path =
                export_path_identity(Path::new(&path)).is_ok_and(|source| source == destination);
            #[cfg(unix)]
            let same_identity = {
                use std::os::unix::fs::MetadataExt;
                destination_metadata.as_ref().is_some_and(|target| {
                    std::fs::metadata(&path).is_ok_and(|source| {
                        source.dev() == target.dev() && source.ino() == target.ino()
                    })
                })
            };
            #[cfg(not(unix))]
            let same_identity = false;
            if same_path || same_identity {
                return Err(SessionError {
                    message: "render destination cannot replace the immutable original".to_owned(),
                });
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn record_render_export(
        &self,
        asset_id: &str,
        adjustment_revision_id: i64,
        output_path: &str,
        format: &str,
        sample_rate: u32,
        channel_count: u32,
        bit_depth: u16,
        frame_count: u64,
        size_bytes: u64,
        integrated_lufs: f32,
        true_peak_dbtp: f32,
        source_disclosure_comment: &str,
    ) -> Result<i64, SessionError> {
        let record = self.verified_render_export_record(
            asset_id,
            adjustment_revision_id,
            output_path,
            format,
            sample_rate,
            channel_count,
            bit_depth,
            frame_count,
            size_bytes,
            integrated_lufs,
            true_peak_dbtp,
        )?;
        self.catalog()
            .with_transaction(|transaction| {
                Self::verify_export_disclosure(
                    transaction,
                    asset_id,
                    0,
                    source_disclosure_comment,
                )?;
                record_render_export(transaction, &record)
            })
            .map(|publication| publication.id)
            .map_err(SessionError::from)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn record_rendered_spectral_working_copy_export(
        &self,
        asset_id: &str,
        adjustment_revision_id: i64,
        working_copy_id: i64,
        rendered_source_path: &str,
        output_path: &str,
        format: &str,
        sample_rate: u32,
        channel_count: u32,
        bit_depth: u16,
        frame_count: u64,
        size_bytes: u64,
        integrated_lufs: f32,
        true_peak_dbtp: f32,
        source_disclosure_comment: &str,
    ) -> Result<i64, SessionError> {
        let asset_id = asset_id.parse::<AssetId>().map_err(|error| SessionError {
            message: error.to_string(),
        })?;
        if working_copy_id <= 0 {
            return Err(SessionError {
                message: "rendered spectral working-copy identity is invalid".to_owned(),
            });
        }
        let rendered_source = Path::new(rendered_source_path);
        let rendered_hash =
            echo_core::hash_file(rendered_source).map_err(|error| SessionError {
                message: error.to_string(),
            })?;
        self.catalog()
            .with_transaction(|transaction| {
                let copy = list_rendered_spectral_working_copies(transaction, asset_id)?
                    .into_iter()
                    .find(|copy| copy.id == working_copy_id)
                    .ok_or_else(|| {
                        echo_catalog::CatalogError::new(
                            echo_catalog::CatalogErrorKind::Constraint,
                            "rendered spectral working copy is unavailable",
                        )
                    })?;
                if !copy.enabled
                    || !matches!(
                        copy.availability,
                        echo_catalog::RenderedSpectralWorkingCopyAvailability::Available
                    )
                    || copy.working_render_content_hash != rendered_hash
                {
                    return Err(echo_catalog::CatalogError::new(
                        echo_catalog::CatalogErrorKind::Constraint,
                        "rendered spectral working copy no longer matches its verified cache",
                    ));
                }
                Ok(())
            })
            .map_err(SessionError::from)?;
        let record = self.verified_render_export_record(
            &asset_id.to_string(),
            adjustment_revision_id,
            output_path,
            format,
            sample_rate,
            channel_count,
            bit_depth,
            frame_count,
            size_bytes,
            integrated_lufs,
            true_peak_dbtp,
        )?;
        self.catalog()
            .with_transaction(|transaction| {
                Self::verify_export_disclosure(
                    transaction,
                    &asset_id.to_string(),
                    0,
                    source_disclosure_comment,
                )?;
                record_rendered_spectral_working_copy_export(transaction, &record, working_copy_id)
            })
            .map(|publication| publication.id)
            .map_err(SessionError::from)
    }

    #[allow(clippy::too_many_arguments)]
    fn verified_render_export_record(
        &self,
        asset_id: &str,
        adjustment_revision_id: i64,
        output_path: &str,
        format: &str,
        sample_rate: u32,
        channel_count: u32,
        bit_depth: u16,
        frame_count: u64,
        size_bytes: u64,
        integrated_lufs: f32,
        true_peak_dbtp: f32,
    ) -> Result<RecordRenderExport, SessionError> {
        let asset_id = asset_id.parse::<AssetId>().map_err(|error| SessionError {
            message: error.to_string(),
        })?;
        let output_path = Path::new(output_path);
        let output_canonical = output_path.canonicalize().map_err(|error| SessionError {
            message: format!(
                "cannot verify rendered file {}: {error}",
                output_path.display()
            ),
        })?;
        let source_path = self
            .catalog()
            .with_transaction(|transaction| find_by_id(transaction, asset_id))
            .map_err(SessionError::from)
            .and_then(|lookup| match lookup {
                AssetLookup::Found(asset) => Ok(asset.original.path),
                AssetLookup::NotFound => Err(SessionError {
                    message: "render source asset is missing".to_owned(),
                }),
            })?;
        if source_path
            .canonicalize()
            .is_ok_and(|source| source == output_canonical)
        {
            return Err(SessionError {
                message: "render destination cannot replace the immutable original".to_owned(),
            });
        }
        let actual_size = output_canonical
            .metadata()
            .map_err(|error| SessionError {
                message: format!(
                    "cannot inspect rendered file {}: {error}",
                    output_canonical.display()
                ),
            })?
            .len();
        if actual_size != size_bytes {
            return Err(SessionError {
                message: "rendered file size does not match publication evidence".to_owned(),
            });
        }
        let content_hash =
            echo_core::hash_file(&output_canonical).map_err(|error| SessionError {
                message: error.to_string(),
            })?;
        let format = match format {
            "wav_pcm16" => RenderExportFormat::WavPcm16,
            "wav_pcm24" => RenderExportFormat::WavPcm24,
            "flac24" => RenderExportFormat::Flac24,
            "wav_float32" => RenderExportFormat::WavFloat32,
            "mp3" => RenderExportFormat::Mp3,
            "aac_m4a" => RenderExportFormat::AacM4a,
            _ => {
                return Err(SessionError {
                    message: "render format is unsupported".to_owned(),
                });
            }
        };
        Ok(RecordRenderExport {
            asset_id,
            adjustment_revision_id,
            output_path: output_canonical,
            format,
            sample_rate,
            channel_count,
            bit_depth,
            frame_count,
            content_hash,
            size_bytes,
            integrated_lufs,
            true_peak_dbtp,
            created_at_millis: now_millis(),
        })
    }
}

#[cfg(test)]
mod tests;
