//! Catalog connection lifecycle and schema ownership.

use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

use rusqlite::{Connection, OptionalExtension};

use crate::{
    error::{CatalogError, CatalogErrorKind},
    schema::{
        ADJUSTMENT_EFFECTS_MIGRATION_SQL, ANCIENT_COMPATIBLE_SCHEMA_VERSION, CatalogSchemaRevision,
        DE_CLICK_MIGRATION_SQL, DE_HUM_MIGRATION_SQL, DELIVERY_FORMATS_MIGRATION_SQL,
        EARLIEST_COMPATIBLE_SCHEMA_VERSION, EDITABLE_EFFECT_CHAIN_SCHEMA_VERSION,
        EFFECT_CHAIN_MIGRATION_SQL, FIXED_EFFECT_CHAIN_SCHEMA_VERSION,
        INITIAL_COMPATIBLE_SCHEMA_VERSION, LEGACY_SCHEMA_VERSION, LONG_AUDIO_MIGRATION_SQL,
        OLDER_COMPATIBLE_SCHEMA_VERSION, OLDEST_COMPATIBLE_SCHEMA_VERSION, PREVIOUS_SCHEMA_VERSION,
        PRIMITIVE_COMPATIBLE_SCHEMA_VERSION, RENDER_EXPORTS_MIGRATION_SQL,
        RESTORATION_CHAIN_MIGRATION_SQL, SCHEMA_IDENTITY, SCHEMA_SQL, SCHEMA_VERSION,
        SEMANTIC_SEARCH_MIGRATION_SQL, USER_ALBUMS_MIGRATION_SQL,
    },
};

/// Durable catalog handle. `SQLite` access is serialized through one mutex: the
/// catalog is a single writer and every reader takes the same lock.
#[derive(Debug)]
pub struct Catalog {
    connection: Mutex<Connection>,
    path: PathBuf,
}

/// Aggregate catalog status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogStats {
    /// Registered asset count.
    pub asset_count: u64,
    /// Append-only analysis record count.
    pub analysis_record_count: u64,
    /// On-disk schema version.
    pub schema_version: CatalogSchemaRevision,
}

/// Opens (creating if needed) a catalog at `path` and verifies the schema.
///
/// # Errors
///
/// Returns [`CatalogErrorKind::SchemaMismatch`] when an existing database does
/// not match the schema this binary supports; callers must not silently
/// downgrade or recreate a foreign catalog.
pub fn open_catalog(path: &Path) -> Result<Catalog, CatalogError> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).map_err(CatalogError::from)?;
    }
    let connection = Connection::open(path)?;
    initialize_schema(&connection)?;
    Ok(Catalog {
        connection: Mutex::new(connection),
        path: path.to_owned(),
    })
}

#[allow(clippy::too_many_lines)] // Explicit dispatch keeps every supported migration visible.
fn initialize_schema(connection: &Connection) -> Result<(), CatalogError> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS catalog_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
    )?;
    let stored_version: Option<String> = connection
        .query_row(
            "SELECT value FROM catalog_meta WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    match stored_version {
        None => {
            connection.execute_batch(SCHEMA_SQL)?;
            connection.execute(
                "INSERT INTO catalog_meta (key, value) VALUES ('schema_version', ?1)",
                [SCHEMA_VERSION.to_string()],
            )?;
            connection.execute(
                "INSERT INTO catalog_meta (key, value) VALUES ('schema_identity', ?1)",
                [SCHEMA_IDENTITY.to_string()],
            )?;
        }
        Some(version)
            if version
                .parse::<CatalogSchemaRevision>()
                .is_ok_and(|revision| revision == SCHEMA_VERSION) =>
        {
            connection.execute_batch(SCHEMA_SQL)?;
            // Identity is descriptive; the canonical revision is authoritative.
        }
        Some(version)
            if version
                .parse::<CatalogSchemaRevision>()
                .is_ok_and(|revision| revision == PREVIOUS_SCHEMA_VERSION) =>
        {
            migrate_restorative_effects_schema(connection)?;
        }
        Some(version)
            if version
                .parse::<CatalogSchemaRevision>()
                .is_ok_and(|revision| revision == EDITABLE_EFFECT_CHAIN_SCHEMA_VERSION) =>
        {
            migrate_editable_effect_chain_schema(connection)?;
        }
        Some(version)
            if version
                .parse::<CatalogSchemaRevision>()
                .is_ok_and(|revision| revision == FIXED_EFFECT_CHAIN_SCHEMA_VERSION) =>
        {
            migrate_effect_chain_schema(connection)?;
        }
        Some(version)
            if version
                .parse::<CatalogSchemaRevision>()
                .is_ok_and(|revision| revision == LEGACY_SCHEMA_VERSION) =>
        {
            migrate_delivery_formats_schema(connection)?;
        }
        Some(version)
            if version
                .parse::<CatalogSchemaRevision>()
                .is_ok_and(|revision| revision == OLDER_COMPATIBLE_SCHEMA_VERSION) =>
        {
            migrate_restoration_chain_schema(connection)?;
        }
        Some(version)
            if version
                .parse::<CatalogSchemaRevision>()
                .is_ok_and(|revision| revision == OLDEST_COMPATIBLE_SCHEMA_VERSION) =>
        {
            migrate_semantic_search_schema(connection)?;
        }
        Some(version)
            if version
                .parse::<CatalogSchemaRevision>()
                .is_ok_and(|revision| revision == ANCIENT_COMPATIBLE_SCHEMA_VERSION) =>
        {
            migrate_long_audio_schema(connection)?;
        }
        Some(version)
            if version
                .parse::<CatalogSchemaRevision>()
                .is_ok_and(|revision| revision == PRIMITIVE_COMPATIBLE_SCHEMA_VERSION) =>
        {
            migrate_user_albums_and_long_audio(connection)?;
        }
        Some(version)
            if version
                .parse::<CatalogSchemaRevision>()
                .is_ok_and(|revision| revision == EARLIEST_COMPATIBLE_SCHEMA_VERSION) =>
        {
            migrate_render_exports_user_albums_and_long_audio(connection)?;
        }
        Some(version)
            if version
                .parse::<CatalogSchemaRevision>()
                .is_ok_and(|revision| revision == INITIAL_COMPATIBLE_SCHEMA_VERSION) =>
        {
            migrate_adjustment_effects_render_exports_user_albums_and_long_audio(connection)?;
        }
        Some(version) => {
            return Err(CatalogError::new(
                CatalogErrorKind::SchemaMismatch,
                format!(
                    "catalog at {} uses schema revision {version}, this binary supports \
                     {SCHEMA_VERSION} ({SCHEMA_IDENTITY})",
                    self_path_display(connection)?
                ),
            ));
        }
    }
    Ok(())
}

fn migrate_restorative_effects_schema(connection: &Connection) -> Result<(), CatalogError> {
    let transaction = connection.unchecked_transaction()?;
    apply_restorative_effects_migration(&transaction)?;
    finish_migration(&transaction)?;
    transaction.commit()?;
    Ok(())
}

fn migrate_editable_effect_chain_schema(connection: &Connection) -> Result<(), CatalogError> {
    let transaction = connection.unchecked_transaction()?;
    apply_restorative_effects_migration(&transaction)?;
    finish_migration(&transaction)?;
    transaction.commit()?;
    Ok(())
}

fn migrate_effect_chain_schema(connection: &Connection) -> Result<(), CatalogError> {
    let transaction = connection.unchecked_transaction()?;
    apply_effect_chain_migration(&transaction)?;
    apply_restorative_effects_migration(&transaction)?;
    finish_migration(&transaction)?;
    transaction.commit()?;
    Ok(())
}

fn migrate_delivery_formats_schema(connection: &Connection) -> Result<(), CatalogError> {
    let transaction = connection.unchecked_transaction()?;
    apply_delivery_formats_migration(&transaction)?;
    apply_effect_chain_migration(&transaction)?;
    apply_restorative_effects_migration(&transaction)?;
    finish_migration(&transaction)?;
    transaction.commit()?;
    Ok(())
}

fn migrate_restoration_chain_schema(connection: &Connection) -> Result<(), CatalogError> {
    let transaction = connection.unchecked_transaction()?;
    apply_restoration_chain_migration(&transaction)?;
    apply_delivery_formats_migration(&transaction)?;
    apply_effect_chain_migration(&transaction)?;
    apply_restorative_effects_migration(&transaction)?;
    finish_migration(&transaction)?;
    transaction.commit()?;
    Ok(())
}

fn migrate_semantic_search_schema(connection: &Connection) -> Result<(), CatalogError> {
    let transaction = connection.unchecked_transaction()?;
    transaction.execute_batch(SEMANTIC_SEARCH_MIGRATION_SQL)?;
    apply_restoration_chain_migration(&transaction)?;
    apply_delivery_formats_migration(&transaction)?;
    apply_effect_chain_migration(&transaction)?;
    apply_restorative_effects_migration(&transaction)?;
    finish_migration(&transaction)?;
    transaction.commit()?;
    Ok(())
}

fn migrate_long_audio_schema(connection: &Connection) -> Result<(), CatalogError> {
    let transaction = connection.unchecked_transaction()?;
    transaction.execute_batch(LONG_AUDIO_MIGRATION_SQL)?;
    transaction.execute_batch(SEMANTIC_SEARCH_MIGRATION_SQL)?;
    apply_restoration_chain_migration(&transaction)?;
    apply_delivery_formats_migration(&transaction)?;
    apply_effect_chain_migration(&transaction)?;
    apply_restorative_effects_migration(&transaction)?;
    finish_migration(&transaction)?;
    transaction.commit()?;
    Ok(())
}

fn migrate_user_albums_and_long_audio(connection: &Connection) -> Result<(), CatalogError> {
    let transaction = connection.unchecked_transaction()?;
    transaction.execute_batch(USER_ALBUMS_MIGRATION_SQL)?;
    transaction.execute_batch(LONG_AUDIO_MIGRATION_SQL)?;
    transaction.execute_batch(SEMANTIC_SEARCH_MIGRATION_SQL)?;
    apply_restoration_chain_migration(&transaction)?;
    apply_delivery_formats_migration(&transaction)?;
    apply_effect_chain_migration(&transaction)?;
    apply_restorative_effects_migration(&transaction)?;
    finish_migration(&transaction)?;
    transaction.commit()?;
    Ok(())
}

fn migrate_render_exports_user_albums_and_long_audio(
    connection: &Connection,
) -> Result<(), CatalogError> {
    let transaction = connection.unchecked_transaction()?;
    transaction.execute_batch(RENDER_EXPORTS_MIGRATION_SQL)?;
    transaction.execute_batch(USER_ALBUMS_MIGRATION_SQL)?;
    transaction.execute_batch(LONG_AUDIO_MIGRATION_SQL)?;
    transaction.execute_batch(SEMANTIC_SEARCH_MIGRATION_SQL)?;
    apply_restoration_chain_migration(&transaction)?;
    apply_delivery_formats_migration(&transaction)?;
    apply_effect_chain_migration(&transaction)?;
    apply_restorative_effects_migration(&transaction)?;
    finish_migration(&transaction)?;
    transaction.commit()?;
    Ok(())
}

fn migrate_adjustment_effects_render_exports_user_albums_and_long_audio(
    connection: &Connection,
) -> Result<(), CatalogError> {
    let transaction = connection.unchecked_transaction()?;
    transaction.execute_batch(ADJUSTMENT_EFFECTS_MIGRATION_SQL)?;
    transaction.execute_batch(RENDER_EXPORTS_MIGRATION_SQL)?;
    transaction.execute_batch(USER_ALBUMS_MIGRATION_SQL)?;
    transaction.execute_batch(LONG_AUDIO_MIGRATION_SQL)?;
    transaction.execute_batch(SEMANTIC_SEARCH_MIGRATION_SQL)?;
    apply_restoration_chain_migration(&transaction)?;
    apply_delivery_formats_migration(&transaction)?;
    apply_effect_chain_migration(&transaction)?;
    apply_restorative_effects_migration(&transaction)?;
    finish_migration(&transaction)?;
    transaction.commit()?;
    Ok(())
}

fn finish_migration(transaction: &rusqlite::Transaction<'_>) -> Result<(), CatalogError> {
    transaction.execute(
        "UPDATE catalog_meta SET value = ?1 WHERE key = 'schema_version'",
        [SCHEMA_VERSION.to_string()],
    )?;
    transaction.execute(
        "INSERT INTO catalog_meta (key, value) VALUES ('schema_identity', ?1) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [SCHEMA_IDENTITY],
    )?;
    Ok(())
}

fn apply_restoration_chain_migration(
    transaction: &rusqlite::Transaction<'_>,
) -> Result<(), CatalogError> {
    let column_count: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('asset_adjustment_revisions') \
         WHERE name = 'restoration_json'",
        [],
        |row| row.get(0),
    )?;
    if column_count == 0 {
        transaction.execute_batch(RESTORATION_CHAIN_MIGRATION_SQL)?;
    }
    Ok(())
}

fn apply_restorative_effects_migration(
    transaction: &rusqlite::Transaction<'_>,
) -> Result<(), CatalogError> {
    let de_hum_column_count: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('asset_adjustment_revisions') \
         WHERE name = 'de_hum_json'",
        [],
        |row| row.get(0),
    )?;
    if de_hum_column_count == 0 {
        transaction.execute_batch(DE_HUM_MIGRATION_SQL)?;
    }

    let de_click_column_count: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('asset_adjustment_revisions') \
         WHERE name = 'de_click_json'",
        [],
        |row| row.get(0),
    )?;
    if de_click_column_count == 0 {
        transaction.execute_batch(DE_CLICK_MIGRATION_SQL)?;
    }
    Ok(())
}

fn apply_effect_chain_migration(
    transaction: &rusqlite::Transaction<'_>,
) -> Result<(), CatalogError> {
    let column_count: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('asset_adjustment_revisions') \
         WHERE name = 'effect_chain_json'",
        [],
        |row| row.get(0),
    )?;
    if column_count == 0 {
        transaction.execute_batch(EFFECT_CHAIN_MIGRATION_SQL)?;
    }
    Ok(())
}

fn apply_delivery_formats_migration(
    transaction: &rusqlite::Transaction<'_>,
) -> Result<(), CatalogError> {
    let supports_flac: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'render_exports' \
         AND sql LIKE '%flac24%'",
        [],
        |row| row.get(0),
    )?;
    if supports_flac == 0 {
        transaction.execute_batch(DELIVERY_FORMATS_MIGRATION_SQL)?;
    }
    Ok(())
}

fn catalog_error_into<E: From<CatalogError>>(error: rusqlite::Error) -> E {
    E::from(CatalogError::from(error))
}

fn self_path_display(connection: &Connection) -> Result<String, CatalogError> {
    let path: String = connection.query_row("PRAGMA database_list", [], |row| row.get(2))?;
    Ok(path)
}

impl Catalog {
    /// Returns the catalog file path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Runs `operation` inside one serialized catalog transaction.
    ///
    /// The caller chooses the error type; catalog failures convert into it.
    ///
    /// # Panics
    ///
    /// Panics when the catalog mutex is poisoned by a prior panic.
    ///
    /// # Errors
    ///
    /// Propagates any transaction or closure failure.
    pub fn with_transaction<T, E>(
        &self,
        operation: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<T, E>,
    ) -> Result<T, E>
    where
        E: From<CatalogError>,
    {
        let connection = self.connection.lock().expect("catalog mutex poisoned");
        let transaction = connection
            .unchecked_transaction()
            .map_err(catalog_error_into)?;
        let result = operation(&transaction)?;
        transaction.commit().map_err(catalog_error_into)?;
        Ok(result)
    }

    /// Reads aggregate catalog status.
    ///
    /// # Panics
    ///
    /// Panics when the catalog mutex is poisoned by a prior panic.
    ///
    /// # Errors
    ///
    /// Propagates any catalog failure from the query.
    pub fn stats(&self) -> Result<CatalogStats, CatalogError> {
        let connection = self.connection.lock().expect("catalog mutex poisoned");
        let asset_count: i64 =
            connection.query_row("SELECT COUNT(*) FROM assets", [], |row| row.get(0))?;
        let analysis_record_count: i64 =
            connection.query_row("SELECT COUNT(*) FROM analysis_records", [], |row| {
                row.get(0)
            })?;
        let schema_version_text: String = connection.query_row(
            "SELECT value FROM catalog_meta WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )?;
        let schema_version = schema_version_text.parse().map_err(|_| {
            CatalogError::new(
                CatalogErrorKind::SchemaMismatch,
                format!("catalog stores malformed schema revision {schema_version_text}"),
            )
        })?;
        Ok(CatalogStats {
            asset_count: u64::try_from(asset_count).expect("count is non-negative"),
            analysis_record_count: u64::try_from(analysis_record_count)
                .expect("count is non-negative"),
            schema_version,
        })
    }
}

#[cfg(test)]
mod tests;
