//! Collection intent and durable media destinations for the desktop.

use super::{LibrarySession, SessionError, now_millis};
use echo_catalog::{JobKind, MaterialImportPayload};
use std::path::Path;

impl LibrarySession {
    pub(crate) fn set_sound_membership(
        &self,
        id: &str,
        memory: bool,
        materials: bool,
        category: &str,
    ) -> Result<(), SessionError> {
        self.catalog
            .with_transaction(|tx| {
                echo_catalog::set_sound_membership(tx, id, memory, materials, category)
            })
            .map_err(SessionError::from)
    }

    pub(crate) fn project_materials(&self, assembly_id: &str) -> Result<Vec<String>, SessionError> {
        self.catalog
            .with_transaction(|tx| echo_catalog::project_material_ids(tx, assembly_id))
            .map_err(SessionError::from)
    }

    pub(crate) fn queue_material_import(
        &self,
        path: &str,
        assembly_id: &str,
        global: bool,
        category: &str,
    ) -> Result<(), SessionError> {
        if self.independent {
            return Err(error(
                "Use explicit project source admission in independent editing",
            ));
        }
        let request = MaterialImportPayload {
            path: Path::new(path).to_owned(),
            assembly_id: assembly_id.to_owned(),
            collect_globally: global,
            category: category.to_owned(),
        };
        request.validate()?;
        if !request.path.is_file() {
            return Err(error("material file is unavailable"));
        }
        let payload = serde_json::to_value(&request).map_err(|e| error(&e.to_string()))?;
        self.catalog
            .with_transaction(|tx| {
                if !assembly_id.is_empty() {
                    let id = assembly_id.parse().map_err(|_| {
                        echo_catalog::CatalogError::new(
                            echo_catalog::CatalogErrorKind::Other,
                            "invalid project",
                        )
                    })?;
                    if echo_catalog::latest_sound_assembly(tx, id)?.is_none() {
                        return Err(echo_catalog::CatalogError::new(
                            echo_catalog::CatalogErrorKind::Other,
                            "project does not exist",
                        ));
                    }
                }
                echo_catalog::enqueue_job(
                    tx,
                    &format!("material-{}", uuid::Uuid::now_v7()),
                    JobKind::ImportMaterial,
                    &payload,
                    now_millis(),
                )
            })
            .map_err(SessionError::from)
    }

    pub(crate) fn memory_output_path(&self, assembly_id: &str) -> Result<String, SessionError> {
        let id: echo_domain::SoundAssemblyId =
            assembly_id.parse().map_err(|_| error("invalid project"))?;
        if self
            .catalog
            .with_transaction(|tx| echo_catalog::latest_sound_assembly(tx, id))?
            .is_none()
        {
            return Err(error("project does not exist"));
        }
        let root = self
            .catalog_path
            .parent()
            .ok_or_else(|| error("catalog has no media directory"))?
            .join("media/memories")
            .join(id.to_string());
        std::fs::create_dir_all(&root)
            .map_err(|e| error(&format!("cannot create memory directory: {e}")))?;
        Ok(root
            .join(format!("{}.wav", uuid::Uuid::now_v7()))
            .to_string_lossy()
            .into_owned())
    }

    pub(crate) fn preserve_assembly_memory(
        &self,
        assembly_id: &str,
        export_id: i64,
    ) -> Result<(), SessionError> {
        // Native rendering has already atomically published and hashed this export.
        // Only managed editions are accepted as collection dependencies.
        let root = self
            .catalog_path
            .parent()
            .ok_or_else(|| error("catalog has no media directory"))?
            .join("media/memories");
        let root = root.canonicalize().map_err(|e| error(&e.to_string()))?;
        self.catalog
            .with_transaction(|tx| -> Result<(), SessionError> {
                let path: String = tx
                    .query_row(
                        "SELECT output_path FROM sound_assembly_exports WHERE id = ?1",
                        [export_id],
                        |r| r.get(0),
                    )
                    .map_err(|e| error(&e.to_string()))?;
                let path = Path::new(&path)
                    .canonicalize()
                    .map_err(|e| error(&e.to_string()))?;
                if !path.starts_with(&root) {
                    return Err(error(
                        "memory edition must be stored in the managed media directory",
                    ));
                }
                echo_catalog::preserve_assembly_memory(tx, assembly_id, export_id, now_millis())?;
                Ok(())
            })
    }
}

fn error(message: &str) -> SessionError {
    SessionError {
        message: message.to_owned(),
    }
}
