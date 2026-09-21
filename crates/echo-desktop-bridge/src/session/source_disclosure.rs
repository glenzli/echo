//! Explicit user declarations, independent of audio-processing revisions.
use super::{LibrarySession, SessionError, now_millis};

impl LibrarySession {
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
