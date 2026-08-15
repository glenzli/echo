//! Cache admission and durable projection for post-effect spectral working
//! copies. The caller owns temporary render bytes; this owner moves their
//! identity into the cache and records only durable provenance in Catalog.

use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use echo_cache::{BlobRole, blob_path, open_blob_store, put_file, verify_blob};
use echo_catalog::{
    CommitRenderedSpectralErase, CreateRenderedSpectralWorkingCopy, RenderedSpectralWorkingCopy,
    commit_rendered_spectral_erase, create_rendered_spectral_working_copy,
    list_rendered_spectral_working_copies, remove_rendered_spectral_working_copy,
    set_rendered_spectral_working_copy_enabled,
};
use echo_domain::AssetId;

use super::{LibrarySession, SessionError, now_millis};

/// A durable working-copy record plus a verified local cache path.
#[derive(Debug, Clone)]
pub(crate) struct ResolvedRenderedSpectralWorkingCopy {
    pub(crate) record: RenderedSpectralWorkingCopy,
    pub(crate) cache_path: PathBuf,
}

impl LibrarySession {
    pub(crate) fn create_rendered_spectral_working_copy(
        &self,
        asset_id: &str,
        adjustment_revision_id: i64,
        rendered_path: &str,
    ) -> Result<ResolvedRenderedSpectralWorkingCopy, SessionError> {
        let asset_id = parse_asset_id(asset_id)?;
        let rendered_path = Path::new(rendered_path);
        let metadata = rendered_path.metadata().map_err(|error| SessionError {
            message: format!("cannot inspect rendered working copy: {error}"),
        })?;
        if !metadata.is_file() || metadata.len() == 0 {
            return Err(SessionError {
                message: "rendered working-copy bytes are unavailable".to_owned(),
            });
        }
        let store = open_blob_store(&self.cache_root).map_err(cache_error)?;
        let (blob, _) =
            put_file(&store, BlobRole::RenderProxy, rendered_path).map_err(cache_error)?;
        let record = self
            .catalog
            .with_transaction(|transaction| {
                create_rendered_spectral_working_copy(
                    transaction,
                    CreateRenderedSpectralWorkingCopy {
                        asset_id,
                        parent_adjustment_revision_id: adjustment_revision_id,
                        parent_render_content_hash: blob.content_hash,
                        created_at_millis: now_millis(),
                    },
                )
            })
            .map_err(SessionError::from)?;
        Ok(ResolvedRenderedSpectralWorkingCopy {
            cache_path: verify_blob(&store, blob.content_hash, blob.size_bytes)
                .map_err(cache_error)?,
            record,
        })
    }

    pub(crate) fn rendered_spectral_working_copies(
        &self,
        asset_id: &str,
    ) -> Result<Vec<ResolvedRenderedSpectralWorkingCopy>, SessionError> {
        let asset_id = parse_asset_id(asset_id)?;
        let copies = self
            .catalog
            .with_transaction(|transaction| {
                list_rendered_spectral_working_copies(transaction, asset_id)
            })
            .map_err(SessionError::from)?;
        let store = open_blob_store(&self.cache_root).map_err(cache_error)?;
        copies
            .into_iter()
            .map(|record| resolve_copy(&store, record))
            .collect()
    }

    pub(crate) fn set_rendered_spectral_working_copy_enabled(
        &self,
        asset_id: &str,
        working_copy_id: i64,
        enabled: bool,
    ) -> Result<(), SessionError> {
        let asset_id = parse_asset_id(asset_id)?;
        self.catalog
            .with_transaction(|transaction| {
                set_rendered_spectral_working_copy_enabled(
                    transaction,
                    asset_id,
                    working_copy_id,
                    enabled,
                )
            })
            .map_err(SessionError::from)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn commit_rendered_spectral_erase(
        &self,
        asset_id: &str,
        working_copy_id: i64,
        rendered_path: &str,
        start_millis: u64,
        end_millis: u64,
        low_hertz: u16,
        high_hertz: u16,
        attenuation_centibels: i16,
        time_feather_millis: u16,
        frequency_feather_hertz: u16,
    ) -> Result<ResolvedRenderedSpectralWorkingCopy, SessionError> {
        let asset_id = parse_asset_id(asset_id)?;
        let rendered_path = Path::new(rendered_path);
        let metadata = rendered_path.metadata().map_err(|error| SessionError {
            message: format!("cannot inspect rendered spectral erase: {error}"),
        })?;
        if !metadata.is_file() || metadata.len() == 0 {
            return Err(SessionError {
                message: "rendered spectral erase bytes are unavailable".to_owned(),
            });
        }
        let store = open_blob_store(&self.cache_root).map_err(cache_error)?;
        let (blob, _) =
            put_file(&store, BlobRole::RenderProxy, rendered_path).map_err(cache_error)?;
        let record = self
            .catalog
            .with_transaction(|transaction| {
                commit_rendered_spectral_erase(
                    transaction,
                    CommitRenderedSpectralErase {
                        asset_id,
                        working_copy_id,
                        rendered_content_hash: blob.content_hash,
                        start_millis,
                        end_millis,
                        low_hertz,
                        high_hertz,
                        attenuation_centibels,
                        time_feather_millis,
                        frequency_feather_hertz,
                    },
                )
            })
            .map_err(SessionError::from)?;
        Ok(ResolvedRenderedSpectralWorkingCopy {
            cache_path: verify_blob(&store, blob.content_hash, blob.size_bytes)
                .map_err(cache_error)?,
            record,
        })
    }

    pub(crate) fn remove_rendered_spectral_working_copy(
        &self,
        asset_id: &str,
        working_copy_id: i64,
    ) -> Result<(), SessionError> {
        let asset_id = parse_asset_id(asset_id)?;
        self.catalog
            .with_transaction(|transaction| {
                remove_rendered_spectral_working_copy(transaction, asset_id, working_copy_id)
            })
            .map_err(SessionError::from)
    }
}

fn parse_asset_id(value: &str) -> Result<AssetId, SessionError> {
    AssetId::from_str(value).map_err(|error| SessionError {
        message: format!("invalid asset id: {error}"),
    })
}

fn resolve_copy(
    store: &echo_cache::BlobStore,
    record: RenderedSpectralWorkingCopy,
) -> Result<ResolvedRenderedSpectralWorkingCopy, SessionError> {
    let cache_path = blob_path(
        echo_cache::store_root(store),
        &record.working_render_content_hash,
    );
    let size_bytes = fs::metadata(&cache_path)
        .map_err(|error| SessionError {
            message: format!("rendered working-copy cache is unavailable: {error}"),
        })?
        .len();
    if size_bytes == 0 {
        return Err(SessionError {
            message: "rendered working-copy cache is empty".to_owned(),
        });
    }
    let cache_path =
        verify_blob(store, record.working_render_content_hash, size_bytes).map_err(cache_error)?;
    Ok(ResolvedRenderedSpectralWorkingCopy { record, cache_path })
}

fn cache_error(error: echo_cache::CacheError) -> SessionError {
    SessionError {
        message: format!("rendered working-copy cache failure: {error}"),
    }
}

#[cfg(test)]
mod tests;
