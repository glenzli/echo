//! Validation and durable publication handoff for desktop offline renders.
//! File hashing stays off the Qt thread and Catalog remains the sole owner of
//! provenance persistence.

use std::path::Path;

use echo_catalog::{
    AssetLookup, RecordRenderExport, RenderExportFormat, find_by_id, record_render_export,
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
        sample_rate: u32,
        channel_count: u32,
        bit_depth: u16,
        frame_count: u64,
        size_bytes: u64,
        integrated_lufs: f32,
        true_peak_dbtp: f32,
    ) -> Result<i64, SessionError> {
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
        let record = RecordRenderExport {
            asset_id,
            adjustment_revision_id,
            output_path: output_canonical,
            format: RenderExportFormat::WavPcm24,
            sample_rate,
            channel_count,
            bit_depth,
            frame_count,
            content_hash,
            size_bytes,
            integrated_lufs,
            true_peak_dbtp,
            created_at_millis: now_millis(),
        };
        self.catalog()
            .with_transaction(|transaction| record_render_export(transaction, &record))
            .map(|publication| publication.id)
            .map_err(SessionError::from)
    }
}

#[cfg(test)]
mod tests;
