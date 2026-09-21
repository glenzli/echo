//! Validation and durable publication handoff for desktop offline renders.
//! File hashing stays off the Qt thread and Catalog remains the sole owner of
//! provenance persistence.

use std::path::Path;

use echo_catalog::{
    AssetLookup, RecordRenderExport, RenderExportFormat, find_by_id,
    list_rendered_spectral_working_copies, record_render_export,
    record_rendered_spectral_working_copy_export,
};
use echo_domain::AssetId;

use crate::session::{LibrarySession, SessionError, now_millis};

impl LibrarySession {
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
