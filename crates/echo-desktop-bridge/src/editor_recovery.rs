//! Read-only recovery summaries. Discovery must not migrate or modify old projects.
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use std::path::Path;

pub(crate) fn summary_json(root: &Path) -> Result<String, String> {
    let connection = Connection::open_with_flags(
        root.join("catalog.sqlite"),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .map_err(|e| e.to_string())?;
    crate::editor_session::verify_private(&connection)?;
    let source_count: i64 = connection
        .query_row("SELECT count(*) FROM assets", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    if source_count == 0 {
        return Ok(String::new());
    }
    let path: String = connection
        .query_row(
            "SELECT path FROM assets ORDER BY imported_at_millis,id LIMIT 1",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let has_assemblies: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='sound_assembly_revisions')",
        [], |row| row.get(0),
    ).map_err(|e| e.to_string())?;
    let assembly_title: Option<String> = if has_assemblies {
        connection
            .query_row(
                "SELECT name FROM sound_assembly_revisions ORDER BY id DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?
    } else {
        None
    };
    let title: String = assembly_title
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| {
            Path::new(&path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        })
        .chars()
        .take(240)
        .collect();
    Ok(serde_json::json!({"title": title, "sourceCount": source_count}).to_string())
}

#[cfg(test)]
mod tests;
