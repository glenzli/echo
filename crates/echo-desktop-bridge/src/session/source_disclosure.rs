//! Explicit user declarations, independent of audio-processing revisions.
use super::{LibrarySession, SessionError, now_millis};

impl LibrarySession {
    pub(crate) fn export_source_disclosure(
        &self,
        id: &str,
        assembly_revision: i64,
    ) -> Result<String, SessionError> {
        self.catalog
            .with_transaction(|tx| {
                Self::export_disclosure_in_transaction(tx, id, assembly_revision)
            })
            .map_err(SessionError::from)
    }

    pub(crate) fn export_disclosure_in_transaction(
        tx: &rusqlite::Transaction<'_>,
        id: &str,
        assembly_revision: i64,
    ) -> Result<String, echo_catalog::CatalogError> {
        let invalid = || {
            echo_catalog::CatalogError::new(
                echo_catalog::CatalogErrorKind::Other,
                "export source does not exist",
            )
        };
        let summary = if assembly_revision > 0 {
            let id = id.parse().map_err(|_| invalid())?;
            let saved = echo_catalog::sound_assembly_at_revision(tx, id, assembly_revision)?
                .ok_or_else(invalid)?;
            echo_catalog::assembly_source_disclosure(
                &saved.assembly,
                &echo_catalog::source_disclosures(tx)?,
            )
        } else {
            let id = id.parse().map_err(|_| invalid())?;
            if !matches!(
                echo_catalog::find_by_id(tx, id)?,
                echo_catalog::AssetLookup::Found(_)
            ) {
                return Err(invalid());
            }
            echo_catalog::asset_source_disclosure(tx, id)?
        };
        Ok(summary.portable_comment())
    }

    pub(crate) fn verify_export_disclosure(
        tx: &rusqlite::Transaction<'_>,
        id: &str,
        assembly_revision: i64,
        comment: &str,
    ) -> Result<(), echo_catalog::CatalogError> {
        if Self::export_disclosure_in_transaction(tx, id, assembly_revision)? != comment {
            return Err(echo_catalog::CatalogError::new(
                echo_catalog::CatalogErrorKind::Constraint,
                "source labels changed or export disclosure is missing; export again",
            ));
        }
        Ok(())
    }

    pub(crate) fn set_source_disclosure(
        &self,
        id: &str,
        expected_revision: i64,
        json: &str,
    ) -> Result<(), SessionError> {
        if json.len() > 96 * 1024 {
            return Err(SessionError {
                message: "source disclosure is too large".into(),
            });
        }
        let id = id.parse().map_err(|_| SessionError {
            message: "invalid source asset".into(),
        })?;
        let spans: Vec<echo_domain::SourceDisclosureSpan> =
            serde_json::from_str(json).map_err(|_| SessionError {
                message: "invalid source disclosure".into(),
            })?;
        self.catalog.with_transaction(|tx| {
            echo_catalog::record_source_disclosure(tx, id, expected_revision, &spans, now_millis())
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
