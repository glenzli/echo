//! Named, reusable processing intent and explicit batch application receipts.
//!
//! Recipe revisions are immutable shared definitions. Applying one materializes
//! an asset-local [`echo_domain::AdjustmentGraph`] revision for every target;
//! later recipe revisions never rewrite sounds that were already processed.

use std::{collections::BTreeSet, str::FromStr};

use echo_domain::{
    AdjustmentGraph, AdjustmentPatch, AssetId, ProcessingMergeMode, ProcessingRecipeId,
    ProcessingRecipeRevision, ProcessingRecipeRevisionId,
};
use rusqlite::{OptionalExtension, Transaction, params};

use crate::{CatalogError, CatalogErrorKind, latest_adjustment_graph, record_adjustment_graph};

const MAX_RECIPE_NAME_CHARACTERS: usize = 80;
const MAX_BATCH_TARGETS: usize = 10_000;

/// One named processing recipe and its current immutable revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessingRecipe {
    pub id: ProcessingRecipeId,
    pub name: String,
    pub current_revision: ProcessingRecipeRevision,
    pub created_at_millis: i64,
    pub updated_at_millis: i64,
}

/// Atomic input for creating a named recipe at revision one.
#[derive(Debug, Clone, Copy)]
pub struct CreateProcessingRecipe<'a> {
    pub name: &'a str,
    pub patch: &'a AdjustmentPatch,
}

/// Stable result for one target in a processing-recipe application batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessingRecipeTargetOutcome {
    Updated,
    Unchanged,
    Failed,
}

/// Durable per-asset evidence from applying one recipe revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessingRecipeTargetReceipt {
    pub asset_id: AssetId,
    pub outcome: ProcessingRecipeTargetOutcome,
    pub previous_adjustment_revision_id: Option<i64>,
    pub resulting_adjustment_revision_id: Option<i64>,
    pub failure_reason: Option<String>,
}

/// Durable receipt for one explicit multi-sound application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessingRecipeApplicationReceipt {
    pub batch_id: i64,
    pub recipe_id: ProcessingRecipeId,
    pub recipe_revision_id: ProcessingRecipeRevisionId,
    pub merge_mode: ProcessingMergeMode,
    pub targets: Vec<ProcessingRecipeTargetReceipt>,
    pub created_at_millis: i64,
}

/// Creates a stable recipe identity and immutable revision one.
///
/// # Errors
///
/// Rejects invalid or duplicate names and propagates persistence failures.
pub fn create_processing_recipe(
    transaction: &Transaction<'_>,
    input: CreateProcessingRecipe<'_>,
    now_millis: i64,
) -> Result<ProcessingRecipe, CatalogError> {
    validate_recipe_name(input.name)?;
    validate_timestamp(now_millis)?;
    ensure_unique_name(transaction, input.name)?;

    let recipe_id = ProcessingRecipeId::new();
    let revision = ProcessingRecipeRevision::new(
        ProcessingRecipeRevisionId::new(),
        recipe_id,
        1,
        input.patch.clone(),
        now_millis,
    )
    .map_err(processing_error)?;
    transaction.execute(
        "INSERT INTO processing_recipes \
         (id, name, created_at_millis, updated_at_millis) VALUES (?1, ?2, ?3, ?3)",
        params![recipe_id.to_string(), input.name, now_millis],
    )?;
    insert_revision(transaction, &revision)?;
    Ok(ProcessingRecipe {
        id: recipe_id,
        name: input.name.to_owned(),
        current_revision: revision,
        created_at_millis: now_millis,
        updated_at_millis: now_millis,
    })
}

/// Appends a new immutable revision and advances the recipe's current
/// projection without changing any previously processed sound.
///
/// # Errors
///
/// Rejects an unknown recipe or invalid revision metadata and propagates
/// persistence failures.
pub fn append_processing_recipe_revision(
    transaction: &Transaction<'_>,
    recipe_id: ProcessingRecipeId,
    patch: &AdjustmentPatch,
    now_millis: i64,
) -> Result<ProcessingRecipeRevision, CatalogError> {
    validate_timestamp(now_millis)?;
    let current = processing_recipe(transaction, recipe_id)?
        .ok_or_else(|| recipe_error("processing recipe does not exist"))?;
    let sequence = current
        .current_revision
        .sequence()
        .checked_add(1)
        .ok_or_else(|| recipe_error("processing recipe revision sequence is exhausted"))?;
    let revision = ProcessingRecipeRevision::new(
        ProcessingRecipeRevisionId::new(),
        recipe_id,
        sequence,
        patch.clone(),
        now_millis,
    )
    .map_err(processing_error)?;
    insert_revision(transaction, &revision)?;
    transaction.execute(
        "UPDATE processing_recipes SET updated_at_millis = ?1 WHERE id = ?2",
        params![now_millis, recipe_id.to_string()],
    )?;
    Ok(revision)
}

/// Lists named recipes with their newest immutable revision.
///
/// # Errors
///
/// Returns a Catalog failure when stored identities or patch JSON are invalid.
pub fn list_processing_recipes(
    transaction: &Transaction<'_>,
) -> Result<Vec<ProcessingRecipe>, CatalogError> {
    let mut statement = transaction.prepare(
        "SELECT recipe.id, recipe.name, recipe.created_at_millis, recipe.updated_at_millis, \
         revision.id, revision.revision_number, revision.patch_json, revision.created_at_millis \
         FROM processing_recipes AS recipe \
         JOIN processing_recipe_revisions AS revision ON revision.recipe_id = recipe.id \
         WHERE revision.revision_number = (SELECT MAX(candidate.revision_number) \
             FROM processing_recipe_revisions AS candidate WHERE candidate.recipe_id = recipe.id) \
         ORDER BY recipe.updated_at_millis DESC, recipe.id",
    )?;
    let rows = statement.query_map([], recipe_row)?;
    rows.map(|row| decode_recipe(row?)).collect()
}

/// Reads one recipe and its current immutable revision.
///
/// # Errors
///
/// Returns a Catalog failure when stored identities or patch JSON are invalid.
pub fn processing_recipe(
    transaction: &Transaction<'_>,
    recipe_id: ProcessingRecipeId,
) -> Result<Option<ProcessingRecipe>, CatalogError> {
    transaction
        .query_row(
            "SELECT recipe.id, recipe.name, recipe.created_at_millis, recipe.updated_at_millis, \
             revision.id, revision.revision_number, revision.patch_json, revision.created_at_millis \
             FROM processing_recipes AS recipe \
             JOIN processing_recipe_revisions AS revision ON revision.recipe_id = recipe.id \
             WHERE recipe.id = ?1 ORDER BY revision.revision_number DESC LIMIT 1",
            [recipe_id.to_string()],
            recipe_row,
        )
        .optional()?
        .map(decode_recipe)
        .transpose()
}

/// Applies the current recipe revision to a de-duplicated asset batch.
///
/// Business failures such as missing assets or incompatible target graphs are
/// persisted per target and do not hide successful siblings. Database failures
/// still abort the caller's transaction, so materialized adjustment revisions
/// and the receipt cannot diverge.
///
/// # Errors
///
/// Rejects an empty or excessive batch, an unknown recipe, invalid persisted
/// data, or a persistence failure.
pub fn apply_processing_recipe(
    transaction: &Transaction<'_>,
    recipe_id: ProcessingRecipeId,
    target_asset_ids: &[AssetId],
    merge_mode: ProcessingMergeMode,
    now_millis: i64,
) -> Result<ProcessingRecipeApplicationReceipt, CatalogError> {
    validate_timestamp(now_millis)?;
    let targets = unique_targets(target_asset_ids)?;
    let recipe = processing_recipe(transaction, recipe_id)?
        .ok_or_else(|| recipe_error("processing recipe does not exist"))?;
    transaction.execute(
        "INSERT INTO processing_recipe_application_batches \
         (recipe_id, recipe_revision_id, merge_mode, created_at_millis) \
         VALUES (?1, ?2, ?3, ?4)",
        params![
            recipe.id.to_string(),
            recipe.current_revision.revision_id().to_string(),
            merge_mode_text(merge_mode),
            now_millis
        ],
    )?;
    let batch_id = transaction.last_insert_rowid();
    let mut receipts = Vec::with_capacity(targets.len());
    for asset_id in targets {
        let receipt = apply_to_target(
            transaction,
            &recipe.current_revision,
            asset_id,
            merge_mode,
            now_millis,
        )?;
        insert_target_receipt(transaction, batch_id, &receipt)?;
        receipts.push(receipt);
    }
    Ok(ProcessingRecipeApplicationReceipt {
        batch_id,
        recipe_id: recipe.id,
        recipe_revision_id: recipe.current_revision.revision_id(),
        merge_mode,
        targets: receipts,
        created_at_millis: now_millis,
    })
}

/// Reads one persisted batch receipt.
///
/// # Errors
///
/// Returns a Catalog failure when stored identities or enum values are invalid.
pub fn processing_recipe_application_receipt(
    transaction: &Transaction<'_>,
    batch_id: i64,
) -> Result<Option<ProcessingRecipeApplicationReceipt>, CatalogError> {
    let header = transaction
        .query_row(
            "SELECT recipe_id, recipe_revision_id, merge_mode, created_at_millis \
             FROM processing_recipe_application_batches WHERE id = ?1",
            [batch_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .optional()?;
    let Some((recipe_id, revision_id, merge_mode, created_at_millis)) = header else {
        return Ok(None);
    };
    let mut statement = transaction.prepare(
        "SELECT asset_id, outcome, previous_adjustment_revision_id, \
         resulting_adjustment_revision_id, failure_reason \
         FROM processing_recipe_application_targets WHERE batch_id = ?1 ORDER BY rowid",
    )?;
    let rows = statement.query_map([batch_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<i64>>(2)?,
            row.get::<_, Option<i64>>(3)?,
            row.get::<_, Option<String>>(4)?,
        ))
    })?;
    let mut targets = Vec::new();
    for row in rows {
        let (asset_id, outcome, previous, resulting, failure_reason) = row?;
        targets.push(ProcessingRecipeTargetReceipt {
            asset_id: parse_asset_id(&asset_id)?,
            outcome: parse_outcome(&outcome)?,
            previous_adjustment_revision_id: previous,
            resulting_adjustment_revision_id: resulting,
            failure_reason,
        });
    }
    Ok(Some(ProcessingRecipeApplicationReceipt {
        batch_id,
        recipe_id: parse_recipe_id(&recipe_id)?,
        recipe_revision_id: parse_revision_id(&revision_id)?,
        merge_mode: parse_merge_mode(&merge_mode)?,
        targets,
        created_at_millis,
    }))
}

type StoredRecipeRow = (String, String, i64, i64, String, i64, String, i64);

fn recipe_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredRecipeRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
    ))
}

fn decode_recipe(stored: StoredRecipeRow) -> Result<ProcessingRecipe, CatalogError> {
    let (recipe_id, name, created_at, updated_at, revision_id, sequence, patch_json, revised_at) =
        stored;
    let recipe_id = parse_recipe_id(&recipe_id)?;
    let patch: AdjustmentPatch = serde_json::from_str(&patch_json).map_err(|error| {
        recipe_error(format!(
            "stored processing recipe patch is invalid: {error}"
        ))
    })?;
    let revision = ProcessingRecipeRevision::new(
        parse_revision_id(&revision_id)?,
        recipe_id,
        u32::try_from(sequence)
            .map_err(|_| recipe_error("stored processing recipe sequence is invalid"))?,
        patch,
        revised_at,
    )
    .map_err(processing_error)?;
    Ok(ProcessingRecipe {
        id: recipe_id,
        name,
        current_revision: revision,
        created_at_millis: created_at,
        updated_at_millis: updated_at,
    })
}

fn insert_revision(
    transaction: &Transaction<'_>,
    revision: &ProcessingRecipeRevision,
) -> Result<(), CatalogError> {
    let patch_json = serde_json::to_string(revision.patch())
        .map_err(|error| recipe_error(format!("cannot encode processing recipe patch: {error}")))?;
    transaction.execute(
        "INSERT INTO processing_recipe_revisions \
         (id, recipe_id, revision_number, patch_json, created_at_millis) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            revision.revision_id().to_string(),
            revision.recipe_id().to_string(),
            i64::from(revision.sequence()),
            patch_json,
            revision.created_at_millis()
        ],
    )?;
    Ok(())
}

fn apply_to_target(
    transaction: &Transaction<'_>,
    revision: &ProcessingRecipeRevision,
    asset_id: AssetId,
    merge_mode: ProcessingMergeMode,
    now_millis: i64,
) -> Result<ProcessingRecipeTargetReceipt, CatalogError> {
    let duration = transaction
        .query_row(
            "SELECT duration_millis FROM assets WHERE id = ?1",
            [asset_id.to_string()],
            |row| row.get::<_, Option<i64>>(0),
        )
        .optional()?;
    let Some(duration) = duration else {
        return Ok(failed_target(asset_id, "asset does not exist"));
    };
    let Some(duration) = duration else {
        return Ok(failed_target(asset_id, "asset duration is unavailable"));
    };
    let Ok(duration) = u64::try_from(duration) else {
        return Ok(failed_target(asset_id, "asset duration is invalid"));
    };
    let previous = latest_adjustment_graph(transaction, asset_id)?;
    let target = match previous {
        Some(revision) => revision.graph,
        None => match AdjustmentGraph::identity(duration) {
            Ok(graph) => graph,
            Err(error) => {
                return Ok(failed_target(
                    asset_id,
                    format!("asset duration cannot form an adjustment graph: {error}"),
                ));
            }
        },
    };
    let graph = match revision.patch().apply_to(target, merge_mode) {
        Ok(graph) => graph,
        Err(error) => {
            return Ok(ProcessingRecipeTargetReceipt {
                asset_id,
                outcome: ProcessingRecipeTargetOutcome::Failed,
                previous_adjustment_revision_id: previous.map(|item| item.revision_id),
                resulting_adjustment_revision_id: None,
                failure_reason: Some(format!(
                    "processing recipe is incompatible with the target: {error}"
                )),
            });
        }
    };
    let saved = record_adjustment_graph(transaction, asset_id, graph, now_millis)?;
    let previous_revision_id = previous.map(|item| item.revision_id);
    let outcome = if previous_revision_id == Some(saved.revision_id) {
        ProcessingRecipeTargetOutcome::Unchanged
    } else {
        ProcessingRecipeTargetOutcome::Updated
    };
    Ok(ProcessingRecipeTargetReceipt {
        asset_id,
        outcome,
        previous_adjustment_revision_id: previous_revision_id,
        resulting_adjustment_revision_id: Some(saved.revision_id),
        failure_reason: None,
    })
}

fn insert_target_receipt(
    transaction: &Transaction<'_>,
    batch_id: i64,
    receipt: &ProcessingRecipeTargetReceipt,
) -> Result<(), CatalogError> {
    transaction.execute(
        "INSERT INTO processing_recipe_application_targets \
         (batch_id, asset_id, outcome, previous_adjustment_revision_id, \
          resulting_adjustment_revision_id, failure_reason) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            batch_id,
            receipt.asset_id.to_string(),
            outcome_text(receipt.outcome),
            receipt.previous_adjustment_revision_id,
            receipt.resulting_adjustment_revision_id,
            receipt.failure_reason
        ],
    )?;
    Ok(())
}

fn unique_targets(targets: &[AssetId]) -> Result<Vec<AssetId>, CatalogError> {
    if targets.is_empty() {
        return Err(recipe_error("processing recipe target batch is empty"));
    }
    if targets.len() > MAX_BATCH_TARGETS {
        return Err(recipe_error("processing recipe target batch is too large"));
    }
    let mut seen = BTreeSet::new();
    Ok(targets
        .iter()
        .copied()
        .filter(|asset_id| seen.insert(*asset_id))
        .collect())
}

fn validate_recipe_name(name: &str) -> Result<(), CatalogError> {
    if name.is_empty()
        || name.trim() != name
        || name.chars().count() > MAX_RECIPE_NAME_CHARACTERS
        || name.chars().any(char::is_control)
    {
        return Err(recipe_error(
            "processing recipe name must be non-empty, trimmed, free of control characters, and at most 80 characters",
        ));
    }
    Ok(())
}

fn validate_timestamp(now_millis: i64) -> Result<(), CatalogError> {
    if now_millis < 0 {
        return Err(recipe_error(
            "processing recipe timestamp must not be negative",
        ));
    }
    Ok(())
}

fn ensure_unique_name(transaction: &Transaction<'_>, name: &str) -> Result<(), CatalogError> {
    let exists = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM processing_recipes WHERE name = ?1 COLLATE NOCASE)",
        [name],
        |row| row.get::<_, bool>(0),
    )?;
    if exists {
        return Err(recipe_error("processing recipe name already exists"));
    }
    Ok(())
}

fn failed_target(asset_id: AssetId, reason: impl Into<String>) -> ProcessingRecipeTargetReceipt {
    ProcessingRecipeTargetReceipt {
        asset_id,
        outcome: ProcessingRecipeTargetOutcome::Failed,
        previous_adjustment_revision_id: None,
        resulting_adjustment_revision_id: None,
        failure_reason: Some(reason.into()),
    }
}

fn parse_recipe_id(value: &str) -> Result<ProcessingRecipeId, CatalogError> {
    ProcessingRecipeId::from_str(value).map_err(|error| {
        recipe_error(format!(
            "invalid stored processing recipe identity: {error}"
        ))
    })
}

fn parse_revision_id(value: &str) -> Result<ProcessingRecipeRevisionId, CatalogError> {
    ProcessingRecipeRevisionId::from_str(value).map_err(|error| {
        recipe_error(format!(
            "invalid stored processing recipe revision identity: {error}"
        ))
    })
}

fn parse_asset_id(value: &str) -> Result<AssetId, CatalogError> {
    AssetId::from_str(value).map_err(|error| {
        recipe_error(format!(
            "invalid stored processing recipe target identity: {error}"
        ))
    })
}

const fn merge_mode_text(mode: ProcessingMergeMode) -> &'static str {
    match mode {
        ProcessingMergeMode::Merge => "merge",
        ProcessingMergeMode::Replace => "replace_processing",
    }
}

fn parse_merge_mode(value: &str) -> Result<ProcessingMergeMode, CatalogError> {
    match value {
        "merge" => Ok(ProcessingMergeMode::Merge),
        "replace_processing" => Ok(ProcessingMergeMode::Replace),
        _ => Err(recipe_error(
            "stored processing recipe merge mode is invalid",
        )),
    }
}

const fn outcome_text(outcome: ProcessingRecipeTargetOutcome) -> &'static str {
    match outcome {
        ProcessingRecipeTargetOutcome::Updated => "updated",
        ProcessingRecipeTargetOutcome::Unchanged => "unchanged",
        ProcessingRecipeTargetOutcome::Failed => "failed",
    }
}

fn parse_outcome(value: &str) -> Result<ProcessingRecipeTargetOutcome, CatalogError> {
    match value {
        "updated" => Ok(ProcessingRecipeTargetOutcome::Updated),
        "unchanged" => Ok(ProcessingRecipeTargetOutcome::Unchanged),
        "failed" => Ok(ProcessingRecipeTargetOutcome::Failed),
        _ => Err(recipe_error(
            "stored processing recipe target outcome is invalid",
        )),
    }
}

fn processing_error(error: impl std::fmt::Display) -> CatalogError {
    recipe_error(error.to_string())
}

fn recipe_error(message: impl Into<String>) -> CatalogError {
    CatalogError::new(CatalogErrorKind::Other, message)
}

#[cfg(test)]
mod tests;
