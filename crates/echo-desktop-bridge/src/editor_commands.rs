//! Typed consumer entry points for saved independent editing sessions.
//!
//! This facade projects identities and timing through the existing Catalog
//! owners. It does not attach a Library session, observe GUI drafts, prepare
//! audio, or publish output. Only `inspect` is a supported editor operation.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

use echo_catalog::{CatalogError, CatalogErrorKind, CatalogSchemaRevision};
use serde_json::{Value, json};

/// Version of this consumer contract, independent of the Catalog schema.
pub const SCHEMA_VERSION: u32 = 1;

/// Supported editor operation; future operations require their own contracts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorOperation {
    Inspect,
}

impl EditorOperation {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Inspect => "inspect",
        }
    }
}

/// Discoverable scope of the production consumer facade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorCapabilities {
    pub schema_version: u32,
    pub catalog_schema_version: CatalogSchemaRevision,
    pub operations: Vec<EditorOperation>,
}

impl EditorCapabilities {
    /// Machine contract with explicit state, time and content-identity scope.
    pub fn to_json(&self) -> Value {
        json!({
            "schema_version": self.schema_version,
            "catalog_schema_version": self.catalog_schema_version.to_string(),
            "operations": self.operations.iter().map(|operation| operation.as_str()).collect::<Vec<_>>(),
            "state_scope": "saved_independent_session",
            "projection": "identity_and_timing",
            "assembly_scope": "active",
            "storage_scope": "quiescent_rollback_journal",
            "revision_scope": "session_catalog",
            "time_unit": "milliseconds",
            "content_hash_algorithm": "blake3-256",
            "content_hash_basis": "catalog_import_identity",
            "source_bytes_verified": false,
            "original_adjustment_revision_id": 0,
        })
    }
}

/// Reports supported operations without accessing any session or user state.
pub fn capabilities() -> EditorCapabilities {
    EditorCapabilities {
        schema_version: SCHEMA_VERSION,
        catalog_schema_version: echo_catalog::supported_schema_revision(),
        operations: vec![EditorOperation::Inspect],
    }
}

/// An explicit, existing private session directory containing `catalog.sqlite`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectRequest {
    pub session_root: PathBuf,
}

/// Latest saved single-source identity. Zero means the unadjusted Original.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetIdentity {
    pub asset_id: String,
    pub content_hash_blake3: String,
    pub size_bytes: u64,
    pub source_duration_millis: Option<u64>,
    pub adjustment_revision_id: i64,
    pub output_duration_millis: Option<u64>,
}

impl AssetIdentity {
    fn to_json(&self) -> Value {
        json!({
            "asset_id": self.asset_id,
            "content_hash_blake3": self.content_hash_blake3,
            "size_bytes": self.size_bytes,
            "source_duration_ms": self.source_duration_millis,
            "adjustment_revision_id": self.adjustment_revision_id,
            "output_duration_ms": self.output_duration_millis,
        })
    }
}

/// A distinct source revision pinned by an assembly, including muted clips.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssemblySourceIdentity {
    pub asset_id: String,
    pub adjustment_revision_id: i64,
    pub content_hash_blake3: String,
}

impl AssemblySourceIdentity {
    fn to_json(&self) -> Value {
        json!({
            "asset_id": self.asset_id,
            "adjustment_revision_id": self.adjustment_revision_id,
            "content_hash_blake3": self.content_hash_blake3,
        })
    }
}

/// Latest saved active assembly and its exact source-revision identities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssemblyIdentity {
    pub assembly_id: String,
    pub name: String,
    pub assembly_revision_id: i64,
    pub revision_number: u32,
    pub duration_millis: u64,
    pub sources: Vec<AssemblySourceIdentity>,
}

impl AssemblyIdentity {
    fn to_json(&self) -> Value {
        json!({
            "assembly_id": self.assembly_id,
            "name": self.name,
            "assembly_revision_id": self.assembly_revision_id,
            "revision_number": self.revision_number,
            "duration_ms": self.duration_millis,
            "sources": self.sources.iter().map(AssemblySourceIdentity::to_json).collect::<Vec<_>>(),
        })
    }
}

/// One consistent read transaction over the saved private Catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectResponse {
    pub capabilities: EditorCapabilities,
    pub assets: Vec<AssetIdentity>,
    pub assemblies: Vec<AssemblyIdentity>,
}

impl InspectResponse {
    /// Versioned JSON used by the real CLI consumer.
    pub fn to_json(&self) -> Value {
        json!({
            "schema_version": SCHEMA_VERSION,
            "ok": true,
            "command": "inspect",
            "capabilities": self.capabilities.to_json(),
            "assets": self.assets.iter().map(AssetIdentity::to_json).collect::<Vec<_>>(),
            "assemblies": self.assemblies.iter().map(AssemblyIdentity::to_json).collect::<Vec<_>>(),
        })
    }
}

/// Stable refusal categories for consumers; no implicit repair is attempted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorCommandErrorCode {
    MissingSession,
    InvalidSessionPath,
    NotIndependentSession,
    UnsupportedCatalogSchema,
    UnsupportedReadOnlyStorage,
    CatalogReadFailed,
}

impl EditorCommandErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MissingSession => "missing_session",
            Self::InvalidSessionPath => "invalid_session_path",
            Self::NotIndependentSession => "not_independent_session",
            Self::UnsupportedCatalogSchema => "unsupported_catalog_schema",
            Self::UnsupportedReadOnlyStorage => "unsupported_read_only_storage",
            Self::CatalogReadFailed => "catalog_read_failed",
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct EditorCommandError {
    pub code: EditorCommandErrorCode,
    pub message: String,
}

impl EditorCommandError {
    fn new(code: EditorCommandErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// Structured refusal emitted on stdout with a nonzero CLI exit status.
    pub fn to_json(&self) -> Value {
        json!({
            "schema_version": SCHEMA_VERSION,
            "ok": false,
            "command": "inspect",
            "error": { "code": self.code.as_str(), "message": self.message },
        })
    }
}

impl From<CatalogError> for EditorCommandError {
    fn from(error: CatalogError) -> Self {
        let code = if error.kind == CatalogErrorKind::SchemaMismatch {
            EditorCommandErrorCode::UnsupportedCatalogSchema
        } else {
            EditorCommandErrorCode::CatalogReadFailed
        };
        Self::new(code, error.message)
    }
}

impl From<echo_catalog::ReadOnlyCatalogError> for EditorCommandError {
    fn from(error: echo_catalog::ReadOnlyCatalogError) -> Self {
        match error {
            echo_catalog::ReadOnlyCatalogError::Catalog(error) => error.into(),
            echo_catalog::ReadOnlyCatalogError::UnsupportedStorage(message) => {
                Self::new(EditorCommandErrorCode::UnsupportedReadOnlyStorage, message)
            }
        }
    }
}

/// Inspects only saved identities and timing in the explicitly named session.
///
/// Source bytes, transcripts, credentials, the global Library and GUI memory
/// are not read. Hashes are recorded import identities, not fresh verification.
///
/// # Errors
///
/// Refuses missing or non-private sessions and every schema requiring creation
/// or migration, and WAL or journal sidecars requiring a writable attachment.
/// Catalog read failures never fall back to a writable attachment.
pub fn inspect(request: &InspectRequest) -> Result<InspectResponse, EditorCommandError> {
    let root_metadata = std::fs::metadata(&request.session_root).map_err(path_error)?;
    let database = request.session_root.join("catalog.sqlite");
    if !root_metadata.is_dir() {
        return Err(EditorCommandError::new(
            EditorCommandErrorCode::InvalidSessionPath,
            "session must be an existing independent editing directory",
        ));
    }
    if !std::fs::metadata(&database).map_err(path_error)?.is_file() {
        return Err(EditorCommandError::new(
            EditorCommandErrorCode::InvalidSessionPath,
            "session catalog must be an existing regular file",
        ));
    }
    let catalog = echo_catalog::open_catalog_read_only(&database)?;
    catalog.with_transaction(|transaction| {
        crate::editor_session::verify_private(transaction).map_err(|message| {
            EditorCommandError::new(EditorCommandErrorCode::NotIndependentSession, message)
        })?;
        let assets = echo_catalog::list_assets(transaction)?
            .into_iter()
            .map(|asset| {
                let adjustment = echo_catalog::latest_adjustment_graph(transaction, asset.id)?;
                Ok(AssetIdentity {
                    asset_id: asset.id.to_string(),
                    content_hash_blake3: asset.original.content_hash.to_string(),
                    size_bytes: asset.original.size_bytes,
                    source_duration_millis: asset.original.duration_millis,
                    adjustment_revision_id: adjustment
                        .as_ref()
                        .map_or(0, |value| value.revision_id),
                    output_duration_millis: adjustment
                        .as_ref()
                        .map_or(asset.original.duration_millis, |value| {
                            Some(value.graph.edit_timeline().output_duration_millis())
                        }),
                })
            })
            .collect::<Result<Vec<_>, CatalogError>>()?;
        let hashes = assets
            .iter()
            .map(|asset| (asset.asset_id.clone(), asset.content_hash_blake3.clone()))
            .collect();
        let assemblies = echo_catalog::list_sound_assemblies(transaction)?
            .into_iter()
            .map(|summary| inspect_assembly(transaction, summary, &hashes))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(InspectResponse {
            capabilities: capabilities(),
            assets,
            assemblies,
        })
    })
}

fn path_error(error: std::io::Error) -> EditorCommandError {
    let code = if error.kind() == std::io::ErrorKind::NotFound {
        EditorCommandErrorCode::MissingSession
    } else {
        EditorCommandErrorCode::CatalogReadFailed
    };
    EditorCommandError::new(code, error.to_string())
}

fn inspect_assembly(
    transaction: &rusqlite::Transaction<'_>,
    summary: echo_catalog::SoundAssemblySummary,
    hashes: &BTreeMap<String, String>,
) -> Result<AssemblyIdentity, EditorCommandError> {
    let revision = echo_catalog::sound_assembly_at_revision(
        transaction,
        summary.assembly_id,
        summary.revision_id,
    )?
    .ok_or_else(|| {
        EditorCommandError::new(
            EditorCommandErrorCode::CatalogReadFailed,
            "saved assembly revision is missing",
        )
    })?;
    let references = revision
        .assembly
        .tracks()
        .iter()
        .flat_map(|track| track.clips())
        .map(|clip| (clip.asset_id().to_string(), clip.adjustment_revision_id()))
        .collect::<BTreeSet<_>>();
    let sources = references
        .into_iter()
        .map(|(asset_id, adjustment_revision_id)| {
            let content_hash_blake3 = hashes
                .get(&asset_id)
                .ok_or_else(|| {
                    EditorCommandError::new(
                        EditorCommandErrorCode::CatalogReadFailed,
                        "saved assembly source is missing",
                    )
                })?
                .clone();
            Ok(AssemblySourceIdentity {
                asset_id,
                adjustment_revision_id,
                content_hash_blake3,
            })
        })
        .collect::<Result<Vec<_>, EditorCommandError>>()?;
    Ok(AssemblyIdentity {
        assembly_id: summary.assembly_id.to_string(),
        name: summary.name,
        assembly_revision_id: summary.revision_id,
        revision_number: summary.revision_number,
        duration_millis: summary.duration_millis,
        sources,
    })
}

#[cfg(test)]
mod tests;
