//! Named, reusable processing intent and explicit batch application receipts.
//!
//! Recipe revisions are immutable shared definitions. Applying one materializes
//! an asset-local [`echo_domain::AdjustmentGraph`] revision for every target;
//! later recipe revisions never rewrite sounds that were already processed.

use std::{collections::BTreeSet, str::FromStr};

use echo_domain::{
    AdjustmentGraph, AdjustmentPatch, AssetId, ProcessingComponent, ProcessingMergeMode,
    ProcessingRecipeId, ProcessingRecipeRevision, ProcessingRecipeRevisionId,
};
use rusqlite::{OptionalExtension, Transaction, params};

use crate::{
    CatalogError, CatalogErrorKind, adjustment_graph::adjustment_graph_at_revision,
    latest_adjustment_graph, record_adjustment_graph,
};

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

/// Stable result for one target in a once-only application revert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessingRecipeRevertTargetOutcome {
    Restored,
    Unchanged,
    Conflict,
    Failed,
}

/// Durable per-asset evidence from reverting one application batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessingRecipeRevertTargetReceipt {
    pub asset_id: AssetId,
    pub outcome: ProcessingRecipeRevertTargetOutcome,
    pub encountered_adjustment_revision_id: Option<i64>,
    pub restored_adjustment_revision_id: Option<i64>,
    pub failure_reason: Option<String>,
}

/// Durable receipt for one once-only application-batch revert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessingRecipeApplicationRevertReceipt {
    pub revert_id: i64,
    pub application_batch_id: i64,
    pub targets: Vec<ProcessingRecipeRevertTargetReceipt>,
    pub created_at_millis: i64,
}

/// Builds a reusable processing patch from one asset's latest local graph, or
/// its duration-derived identity graph when it has never been adjusted.
///
/// # Errors
///
/// Rejects missing assets, unknown durations, invalid component selections,
/// corrupt stored adjustments, or persistence failures.
pub fn processing_recipe_patch_from_asset(
    transaction: &Transaction<'_>,
    asset_id: AssetId,
    components: &[ProcessingComponent],
) -> Result<AdjustmentPatch, CatalogError> {
    let graph = match latest_adjustment_graph(transaction, asset_id)? {
        Some(revision) => revision.graph,
        None => identity_graph_for_asset(transaction, asset_id)?.map_err(recipe_error)?,
    };
    AdjustmentPatch::from_graph(graph, components).map_err(processing_error)
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
    ensure_unique_name(transaction, input.name, None)?;

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

/// Renames an active recipe without changing its stable identity or revisions.
///
/// # Errors
///
/// Rejects invalid or duplicate names, archived or unknown recipes, and
/// persistence failures.
pub fn rename_processing_recipe(
    transaction: &Transaction<'_>,
    recipe_id: ProcessingRecipeId,
    name: &str,
    now_millis: i64,
) -> Result<ProcessingRecipe, CatalogError> {
    validate_recipe_name(name)?;
    validate_timestamp(now_millis)?;
    let mut recipe = processing_recipe(transaction, recipe_id)?
        .ok_or_else(|| recipe_error("active processing recipe does not exist"))?;
    if recipe.name == name {
        return Ok(recipe);
    }
    ensure_unique_name(transaction, name, Some(recipe_id))?;
    transaction.execute(
        "UPDATE processing_recipes SET name = ?1, updated_at_millis = ?2 \
         WHERE id = ?3 AND archived_at_millis IS NULL",
        params![name, now_millis, recipe_id.to_string()],
    )?;
    name.clone_into(&mut recipe.name);
    recipe.updated_at_millis = now_millis;
    Ok(recipe)
}

/// Archives a recipe without deleting its immutable revisions or application
/// receipts. Repeating the operation is idempotent.
///
/// # Errors
///
/// Rejects an unknown recipe or invalid timestamp and propagates persistence
/// failures.
pub fn archive_processing_recipe(
    transaction: &Transaction<'_>,
    recipe_id: ProcessingRecipeId,
    now_millis: i64,
) -> Result<bool, CatalogError> {
    validate_timestamp(now_millis)?;
    let changed = transaction.execute(
        "UPDATE processing_recipes SET archived_at_millis = ?1, updated_at_millis = ?1 \
         WHERE id = ?2 AND archived_at_millis IS NULL",
        params![now_millis, recipe_id.to_string()],
    )? > 0;
    if changed {
        return Ok(true);
    }
    let exists = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM processing_recipes WHERE id = ?1)",
        [recipe_id.to_string()],
        |row| row.get::<_, bool>(0),
    )?;
    if exists {
        Ok(false)
    } else {
        Err(recipe_error("processing recipe does not exist"))
    }
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
         WHERE recipe.archived_at_millis IS NULL \
         AND revision.revision_number = (SELECT MAX(candidate.revision_number) \
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
             WHERE recipe.id = ?1 AND recipe.archived_at_millis IS NULL \
             ORDER BY revision.revision_number DESC LIMIT 1",
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

/// Reverts an application batch at most once using optimistic revision guards.
///
/// A target is restored only when the application originally updated it and
/// its current adjustment revision still equals that application's result.
/// Repeating this call returns the durable first receipt without writing
/// another adjustment revision.
///
/// # Errors
///
/// Rejects an unknown application batch or invalid timestamp and propagates
/// persistence failures. Database failures abort the caller's transaction.
pub fn revert_processing_recipe_application(
    transaction: &Transaction<'_>,
    application_batch_id: i64,
    now_millis: i64,
) -> Result<ProcessingRecipeApplicationRevertReceipt, CatalogError> {
    validate_timestamp(now_millis)?;
    if let Some(receipt) =
        processing_recipe_application_revert_receipt(transaction, application_batch_id)?
    {
        return Ok(receipt);
    }
    let application = processing_recipe_application_receipt(transaction, application_batch_id)?
        .ok_or_else(|| recipe_error("processing recipe application does not exist"))?;
    transaction.execute(
        "INSERT INTO processing_recipe_application_reverts \
         (application_batch_id, created_at_millis) VALUES (?1, ?2)",
        params![application_batch_id, now_millis],
    )?;
    let revert_id = transaction.last_insert_rowid();
    let mut targets = Vec::with_capacity(application.targets.len());
    for application_target in &application.targets {
        let target = revert_application_target(transaction, application_target, now_millis)?;
        insert_revert_target_receipt(transaction, revert_id, &target)?;
        targets.push(target);
    }
    Ok(ProcessingRecipeApplicationRevertReceipt {
        revert_id,
        application_batch_id,
        targets,
        created_at_millis: now_millis,
    })
}

/// Reads the durable revert receipt for an application batch.
///
/// # Errors
///
/// Returns a Catalog failure when stored identities or outcome values are
/// invalid.
pub fn processing_recipe_application_revert_receipt(
    transaction: &Transaction<'_>,
    application_batch_id: i64,
) -> Result<Option<ProcessingRecipeApplicationRevertReceipt>, CatalogError> {
    let header = transaction
        .query_row(
            "SELECT id, created_at_millis FROM processing_recipe_application_reverts \
             WHERE application_batch_id = ?1",
            [application_batch_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()?;
    let Some((revert_id, created_at_millis)) = header else {
        return Ok(None);
    };
    let mut statement = transaction.prepare(
        "SELECT asset_id, outcome, encountered_adjustment_revision_id, \
         restored_adjustment_revision_id, failure_reason \
         FROM processing_recipe_application_revert_targets \
         WHERE revert_id = ?1 ORDER BY rowid",
    )?;
    let rows = statement.query_map([revert_id], |row| {
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
        let (asset_id, outcome, encountered, restored, failure_reason) = row?;
        targets.push(ProcessingRecipeRevertTargetReceipt {
            asset_id: parse_asset_id(&asset_id)?,
            outcome: parse_revert_outcome(&outcome)?,
            encountered_adjustment_revision_id: encountered,
            restored_adjustment_revision_id: restored,
            failure_reason,
        });
    }
    Ok(Some(ProcessingRecipeApplicationRevertReceipt {
        revert_id,
        application_batch_id,
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

fn revert_application_target(
    transaction: &Transaction<'_>,
    application: &ProcessingRecipeTargetReceipt,
    now_millis: i64,
) -> Result<ProcessingRecipeRevertTargetReceipt, CatalogError> {
    let current = latest_adjustment_graph(transaction, application.asset_id)?;
    let encountered = current.map(|revision| revision.revision_id);
    if application.outcome != ProcessingRecipeTargetOutcome::Updated {
        return Ok(ProcessingRecipeRevertTargetReceipt {
            asset_id: application.asset_id,
            outcome: ProcessingRecipeRevertTargetOutcome::Unchanged,
            encountered_adjustment_revision_id: encountered,
            restored_adjustment_revision_id: None,
            failure_reason: None,
        });
    }
    let Some(expected_revision_id) = application.resulting_adjustment_revision_id else {
        return Ok(failed_revert_target(
            application.asset_id,
            encountered,
            "application result revision is unavailable",
        ));
    };
    let Some(current) = current else {
        return Ok(failed_revert_target(
            application.asset_id,
            None,
            "current adjustment revision is unavailable",
        ));
    };
    if current.revision_id != expected_revision_id {
        return Ok(ProcessingRecipeRevertTargetReceipt {
            asset_id: application.asset_id,
            outcome: ProcessingRecipeRevertTargetOutcome::Conflict,
            encountered_adjustment_revision_id: Some(current.revision_id),
            restored_adjustment_revision_id: None,
            failure_reason: None,
        });
    }
    let restore_graph = match application.previous_adjustment_revision_id {
        Some(previous_revision_id) => {
            let previous = adjustment_graph_at_revision(
                transaction,
                application.asset_id,
                previous_revision_id,
            )?;
            let Some(previous) = previous else {
                return Ok(failed_revert_target(
                    application.asset_id,
                    Some(current.revision_id),
                    "previous adjustment revision is unavailable",
                ));
            };
            previous.graph
        }
        None => match identity_graph_for_asset(transaction, application.asset_id)? {
            Ok(graph) => graph,
            Err(reason) => {
                return Ok(failed_revert_target(
                    application.asset_id,
                    Some(current.revision_id),
                    reason,
                ));
            }
        },
    };
    let restored =
        record_adjustment_graph(transaction, application.asset_id, restore_graph, now_millis)?;
    Ok(ProcessingRecipeRevertTargetReceipt {
        asset_id: application.asset_id,
        outcome: ProcessingRecipeRevertTargetOutcome::Restored,
        encountered_adjustment_revision_id: Some(current.revision_id),
        restored_adjustment_revision_id: Some(restored.revision_id),
        failure_reason: None,
    })
}

fn identity_graph_for_asset(
    transaction: &Transaction<'_>,
    asset_id: AssetId,
) -> Result<Result<AdjustmentGraph, String>, CatalogError> {
    let duration = transaction
        .query_row(
            "SELECT duration_millis FROM assets WHERE id = ?1",
            [asset_id.to_string()],
            |row| row.get::<_, Option<i64>>(0),
        )
        .optional()?;
    let Some(duration) = duration else {
        return Ok(Err("asset does not exist".to_owned()));
    };
    let Some(duration) = duration else {
        return Ok(Err("asset duration is unavailable".to_owned()));
    };
    let Ok(duration) = u64::try_from(duration) else {
        return Ok(Err("asset duration is invalid".to_owned()));
    };
    Ok(AdjustmentGraph::identity(duration).map_err(|error| {
        format!("asset duration cannot form an identity adjustment graph: {error}")
    }))
}

fn insert_revert_target_receipt(
    transaction: &Transaction<'_>,
    revert_id: i64,
    receipt: &ProcessingRecipeRevertTargetReceipt,
) -> Result<(), CatalogError> {
    transaction.execute(
        "INSERT INTO processing_recipe_application_revert_targets \
         (revert_id, asset_id, outcome, encountered_adjustment_revision_id, \
          restored_adjustment_revision_id, failure_reason) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            revert_id,
            receipt.asset_id.to_string(),
            revert_outcome_text(receipt.outcome),
            receipt.encountered_adjustment_revision_id,
            receipt.restored_adjustment_revision_id,
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

fn ensure_unique_name(
    transaction: &Transaction<'_>,
    name: &str,
    except_recipe_id: Option<ProcessingRecipeId>,
) -> Result<(), CatalogError> {
    let exists = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM processing_recipes \
         WHERE name = ?1 COLLATE NOCASE AND id != ?2)",
        params![
            name,
            except_recipe_id
                .map(|id| id.to_string())
                .unwrap_or_default()
        ],
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

fn failed_revert_target(
    asset_id: AssetId,
    encountered_adjustment_revision_id: Option<i64>,
    reason: impl Into<String>,
) -> ProcessingRecipeRevertTargetReceipt {
    ProcessingRecipeRevertTargetReceipt {
        asset_id,
        outcome: ProcessingRecipeRevertTargetOutcome::Failed,
        encountered_adjustment_revision_id,
        restored_adjustment_revision_id: None,
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

const fn revert_outcome_text(outcome: ProcessingRecipeRevertTargetOutcome) -> &'static str {
    match outcome {
        ProcessingRecipeRevertTargetOutcome::Restored => "restored",
        ProcessingRecipeRevertTargetOutcome::Unchanged => "unchanged",
        ProcessingRecipeRevertTargetOutcome::Conflict => "conflict",
        ProcessingRecipeRevertTargetOutcome::Failed => "failed",
    }
}

fn parse_revert_outcome(value: &str) -> Result<ProcessingRecipeRevertTargetOutcome, CatalogError> {
    match value {
        "restored" => Ok(ProcessingRecipeRevertTargetOutcome::Restored),
        "unchanged" => Ok(ProcessingRecipeRevertTargetOutcome::Unchanged),
        "conflict" => Ok(ProcessingRecipeRevertTargetOutcome::Conflict),
        "failed" => Ok(ProcessingRecipeRevertTargetOutcome::Failed),
        _ => Err(recipe_error(
            "stored processing recipe revert target outcome is invalid",
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
