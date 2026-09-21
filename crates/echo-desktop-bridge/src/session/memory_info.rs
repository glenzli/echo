//! Personal memory information API shared by the library and portable editor.
use super::{LibrarySession, SessionError, now_millis};

impl LibrarySession {
    pub(crate) fn memory_info_json(
        &self,
        id: &str,
        assembly: bool,
    ) -> Result<String, SessionError> {
        let value = self
            .catalog
            .with_transaction(|tx| echo_catalog::memory_info(tx, id, assembly))?;
        serde_json::to_string(&value).map_err(|e| SessionError {
            message: e.to_string(),
        })
    }
    pub(crate) fn set_memory_info(
        &self,
        id: &str,
        assembly: bool,
        expected: i64,
        json: &str,
    ) -> Result<(), SessionError> {
        if json.len() > 300_000 {
            return Err(SessionError {
                message: "memory information is too large".into(),
            });
        }
        let info: echo_domain::MemoryInfo =
            serde_json::from_str(json).map_err(|e| SessionError {
                message: e.to_string(),
            })?;
        self.catalog.with_transaction(|tx| {
            echo_catalog::record_memory_info(tx, id, assembly, expected, info, now_millis())
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
