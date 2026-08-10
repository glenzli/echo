//! Desktop processing-recipe lifecycle and batch projection.
//!
//! Shared recipe definitions stay in the Catalog. This owner converts them
//! to desktop wire values and materializes selected source processing without
//! leaking recipe lookup into the real-time audio path.

use std::str::FromStr;

use echo_domain::{AssetId, ProcessingComponent, ProcessingMergeMode, ProcessingRecipeId};

use super::{LibrarySession, SessionError, now_millis};
use crate::ffi::{
    ProcessingRecipeApplyReceiptWire, ProcessingRecipeRevertReceiptWire,
    ProcessingRecipeRevertTargetResultWire, ProcessingRecipeTargetResultWire, ProcessingRecipeWire,
};

impl LibrarySession {
    /// Lists active recipes at their newest immutable revisions.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when stored recipe evidence is invalid or the
    /// Catalog projection fails.
    pub fn processing_recipes(&self) -> Result<Vec<ProcessingRecipeWire>, SessionError> {
        let recipes = self
            .catalog
            .with_transaction(echo_catalog::list_processing_recipes)
            .map_err(SessionError::from)?;
        Ok(recipes
            .into_iter()
            .map(|recipe| ProcessingRecipeWire {
                id: recipe.id.to_string(),
                name: recipe.name,
                revision_id: recipe.current_revision.revision_id().to_string(),
                revision_number: recipe.current_revision.sequence(),
                components: recipe
                    .current_revision
                    .patch()
                    .components()
                    .iter()
                    .copied()
                    .map(ProcessingComponent::wire_value)
                    .collect(),
                updated_at_millis: recipe.updated_at_millis,
            })
            .collect())
    }

    /// Saves selected processing from one asset's current persisted graph.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] for an unknown asset, unavailable duration,
    /// invalid component wire value, invalid name, or Catalog failure.
    pub fn create_processing_recipe(
        &self,
        name: &str,
        source_asset_id: &str,
        component_values: &[u8],
    ) -> Result<String, SessionError> {
        let (asset_id, components) = source_selection(source_asset_id, component_values)?;
        self.catalog
            .with_transaction(|transaction| {
                let patch = echo_catalog::processing_recipe_patch_from_asset(
                    transaction,
                    asset_id,
                    &components,
                )?;
                echo_catalog::create_processing_recipe(
                    transaction,
                    echo_catalog::CreateProcessingRecipe {
                        name,
                        patch: &patch,
                    },
                    now_millis(),
                )
            })
            .map(|recipe| recipe.id.to_string())
            .map_err(SessionError::from)
    }

    /// Renames an active recipe without changing its immutable revisions.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] for an invalid identity, name, archived
    /// recipe, or Catalog failure.
    pub fn rename_processing_recipe(
        &self,
        recipe_id: &str,
        name: &str,
    ) -> Result<(), SessionError> {
        let recipe_id = parse_recipe_id(recipe_id)?;
        self.catalog
            .with_transaction(|transaction| {
                echo_catalog::rename_processing_recipe(transaction, recipe_id, name, now_millis())
            })
            .map(|_| ())
            .map_err(SessionError::from)
    }

    /// Appends selected processing from one sound as a new recipe revision.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] for invalid identities, components, source
    /// adjustment evidence, archived recipes, or Catalog failures.
    pub fn update_processing_recipe(
        &self,
        recipe_id: &str,
        source_asset_id: &str,
        component_values: &[u8],
    ) -> Result<u32, SessionError> {
        let recipe_id = parse_recipe_id(recipe_id)?;
        let (asset_id, components) = source_selection(source_asset_id, component_values)?;
        self.catalog
            .with_transaction(|transaction| {
                let patch = echo_catalog::processing_recipe_patch_from_asset(
                    transaction,
                    asset_id,
                    &components,
                )?;
                echo_catalog::append_processing_recipe_revision(
                    transaction,
                    recipe_id,
                    &patch,
                    now_millis(),
                )
            })
            .map(|revision| revision.sequence())
            .map_err(SessionError::from)
    }

    /// Archives a recipe while preserving revisions and application evidence.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] for an invalid identity, unknown recipe, or
    /// Catalog failure.
    pub fn archive_processing_recipe(&self, recipe_id: &str) -> Result<(), SessionError> {
        let recipe_id = parse_recipe_id(recipe_id)?;
        self.catalog
            .with_transaction(|transaction| {
                echo_catalog::archive_processing_recipe(transaction, recipe_id, now_millis())
            })
            .map(|_| ())
            .map_err(SessionError::from)
    }

    /// Applies one recipe revision to explicit targets and returns its durable
    /// per-sound receipt.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when an identity or merge mode is invalid, or
    /// when the Catalog cannot atomically persist the batch.
    pub fn apply_processing_recipe(
        &self,
        recipe_id: &str,
        target_asset_ids: &[String],
        merge_mode: u8,
    ) -> Result<ProcessingRecipeApplyReceiptWire, SessionError> {
        let recipe_id = parse_recipe_id(recipe_id)?;
        let targets = target_asset_ids
            .iter()
            .map(|asset_id| {
                AssetId::from_str(asset_id).map_err(|error| SessionError {
                    message: format!("invalid target asset id {asset_id}: {error}"),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let merge_mode = match merge_mode {
            0 => ProcessingMergeMode::Merge,
            1 => ProcessingMergeMode::Replace,
            _ => {
                return Err(SessionError {
                    message: "processing recipe merge mode is invalid".to_owned(),
                });
            }
        };
        let receipt = self
            .catalog
            .with_transaction(|transaction| {
                echo_catalog::apply_processing_recipe(
                    transaction,
                    recipe_id,
                    &targets,
                    merge_mode,
                    now_millis(),
                )
            })
            .map_err(SessionError::from)?;
        Ok(apply_receipt_wire(receipt))
    }

    /// Reverts one application batch without overwriting later sound edits.
    ///
    /// Repeating the operation returns the first durable receipt and does not
    /// append more adjustment revisions.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] for an invalid or unknown application batch,
    /// corrupt history, or Catalog failure.
    pub fn revert_processing_recipe_application(
        &self,
        batch_id: &str,
    ) -> Result<ProcessingRecipeRevertReceiptWire, SessionError> {
        let application_batch_id = batch_id.parse::<i64>().map_err(|error| SessionError {
            message: format!("invalid processing recipe application batch {batch_id}: {error}"),
        })?;
        if application_batch_id <= 0 {
            return Err(SessionError {
                message: "processing recipe application batch must be positive".to_owned(),
            });
        }
        let receipt = self
            .catalog
            .with_transaction(|transaction| {
                echo_catalog::revert_processing_recipe_application(
                    transaction,
                    application_batch_id,
                    now_millis(),
                )
            })
            .map_err(SessionError::from)?;
        Ok(revert_receipt_wire(receipt))
    }
}

fn source_selection(
    source_asset_id: &str,
    component_values: &[u8],
) -> Result<(AssetId, Vec<ProcessingComponent>), SessionError> {
    let asset_id = AssetId::from_str(source_asset_id).map_err(|error| SessionError {
        message: format!("invalid source asset id {source_asset_id}: {error}"),
    })?;
    let components = component_values
        .iter()
        .copied()
        .map(|value| {
            ProcessingComponent::from_wire_value(value).map_err(|error| SessionError {
                message: error.to_string(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok((asset_id, components))
}

fn parse_recipe_id(recipe_id: &str) -> Result<ProcessingRecipeId, SessionError> {
    ProcessingRecipeId::from_str(recipe_id).map_err(|error| SessionError {
        message: format!("invalid processing recipe id {recipe_id}: {error}"),
    })
}

fn apply_receipt_wire(
    receipt: echo_catalog::ProcessingRecipeApplicationReceipt,
) -> ProcessingRecipeApplyReceiptWire {
    let mut updated_count = 0_u64;
    let mut unchanged_count = 0_u64;
    let mut failed_count = 0_u64;
    let results = receipt
        .targets
        .into_iter()
        .map(|target| {
            let outcome = match target.outcome {
                echo_catalog::ProcessingRecipeTargetOutcome::Updated => {
                    updated_count += 1;
                    "updated"
                }
                echo_catalog::ProcessingRecipeTargetOutcome::Unchanged => {
                    unchanged_count += 1;
                    "unchanged"
                }
                echo_catalog::ProcessingRecipeTargetOutcome::Failed => {
                    failed_count += 1;
                    "failed"
                }
            };
            ProcessingRecipeTargetResultWire {
                asset_id: target.asset_id.to_string(),
                outcome: outcome.to_owned(),
                adjustment_revision: target.resulting_adjustment_revision_id.unwrap_or(0),
                error: target.failure_reason.unwrap_or_default(),
            }
        })
        .collect();
    ProcessingRecipeApplyReceiptWire {
        batch_id: receipt.batch_id.to_string(),
        recipe_revision_id: receipt.recipe_revision_id.to_string(),
        updated_count,
        unchanged_count,
        failed_count,
        results,
    }
}

fn revert_receipt_wire(
    receipt: echo_catalog::ProcessingRecipeApplicationRevertReceipt,
) -> ProcessingRecipeRevertReceiptWire {
    let mut restored_count = 0_u64;
    let mut unchanged_count = 0_u64;
    let mut conflict_count = 0_u64;
    let mut failed_count = 0_u64;
    let results = receipt
        .targets
        .into_iter()
        .map(|target| {
            let outcome = match target.outcome {
                echo_catalog::ProcessingRecipeRevertTargetOutcome::Restored => {
                    restored_count += 1;
                    "restored"
                }
                echo_catalog::ProcessingRecipeRevertTargetOutcome::Unchanged => {
                    unchanged_count += 1;
                    "unchanged"
                }
                echo_catalog::ProcessingRecipeRevertTargetOutcome::Conflict => {
                    conflict_count += 1;
                    "conflict"
                }
                echo_catalog::ProcessingRecipeRevertTargetOutcome::Failed => {
                    failed_count += 1;
                    "failed"
                }
            };
            ProcessingRecipeRevertTargetResultWire {
                asset_id: target.asset_id.to_string(),
                outcome: outcome.to_owned(),
                adjustment_revision: target.restored_adjustment_revision_id.unwrap_or(0),
                error: target.failure_reason.unwrap_or_default(),
            }
        })
        .collect();
    ProcessingRecipeRevertReceiptWire {
        revert_id: receipt.revert_id.to_string(),
        application_batch_id: receipt.application_batch_id.to_string(),
        restored_count,
        unchanged_count,
        conflict_count,
        failed_count,
        results,
    }
}
