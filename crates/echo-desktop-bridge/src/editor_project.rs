//! Portable editor project snapshots. Streaming file chunks keep memory bounded;
//! owned relative paths remain usable when a project is moved to another machine.

use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use std::{
    fs,
    io::{Read, Write},
    path::{Component, Path},
};

const FORMAT: &str = "echo-independent-project@1";
const CHUNK: usize = 1024 * 1024;

fn checked_path(value: &str) -> Result<&Path, String> {
    let path = Path::new(value);
    if !matches!(path.components().next(), Some(Component::Normal(v)) if v == "media" || v == "cache" || v == "impulse-responses")
        || path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
        || value.contains('\\')
    {
        return Err("invalid project resource path".into());
    }
    Ok(path)
}

fn readonly(path: &Path) -> Result<Connection, String> {
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|e| e.to_string())
}

fn verify(connection: &Connection) -> Result<(), String> {
    let format: Option<String> = connection
        .query_row(
            "SELECT value FROM catalog_meta WHERE key='editor_project_format'",
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if format.as_deref() != Some(FORMAT) {
        return Err("not a supported Echo editing project".into());
    }
    crate::editor_session::verify_private(connection)?;
    verify_source_identity(connection)
}

fn verify_source_identity(connection: &Connection) -> Result<(), String> {
    let invalid: i64 = connection.query_row("SELECT COUNT(*) FROM assets a LEFT JOIN editor_project_files f ON f.path=a.path WHERE f.digest IS NULL OR f.digest != a.content_hash", [], |r| r.get(0)).map_err(|e| e.to_string())?;
    if invalid != 0 {
        return Err("project source identity does not match its preserved audio".into());
    }
    Ok(())
}

pub(crate) fn save(root: &Path, destination: &Path) -> Result<(), String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let parent = destination
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent = parent.canonicalize().map_err(|e| e.to_string())?;
    if parent.starts_with(&root) {
        return Err("save the project outside its working directory".into());
    }
    if destination.exists() {
        verify(&readonly(destination)?)?;
    }
    let staging = parent.join(format!(".echo-project-{}.partial", uuid::Uuid::new_v4()));
    let result = (|| -> Result<(), String> {
        let input = readonly(&root.join("catalog.sqlite"))?;
        crate::editor_session::verify_private(&input)?;
        input
            .backup(rusqlite::MAIN_DB, &staging, None)
            .map_err(|e| e.to_string())?;
        let mut output = Connection::open(&staging).map_err(|e| e.to_string())?;
        let tx = output.transaction().map_err(|e| e.to_string())?;
        tx.execute_batch("CREATE TABLE editor_project_files(path TEXT PRIMARY KEY, size INTEGER NOT NULL, digest TEXT NOT NULL); CREATE TABLE editor_project_chunks(path TEXT NOT NULL, ordinal INTEGER NOT NULL, data BLOB NOT NULL, PRIMARY KEY(path, ordinal));").map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT OR REPLACE INTO catalog_meta(key,value) VALUES('editor_project_format',?1)",
            [FORMAT],
        )
        .map_err(|e| e.to_string())?;
        let mut files = Vec::new();
        for name in ["media", "cache", "impulse-responses"] {
            let directory = root.join(name);
            if directory.exists() {
                collect(&directory, &mut files)?;
            }
        }
        for file in files {
            let relative = file
                .strip_prefix(&root)
                .map_err(|e| e.to_string())?
                .to_str()
                .ok_or("project path is not UTF-8")?
                .replace('\\', "/");
            checked_path(&relative)?;
            let digest = echo_core::hash_file(&file)
                .map_err(|e| e.to_string())?
                .to_string();
            let size = file.metadata().map_err(|e| e.to_string())?.len();
            tx.execute(
                "INSERT INTO editor_project_files VALUES(?1,?2,?3)",
                params![
                    relative,
                    i64::try_from(size).map_err(|e| e.to_string())?,
                    digest
                ],
            )
            .map_err(|e| e.to_string())?;
            let mut input = fs::File::open(file).map_err(|e| e.to_string())?;
            let mut buffer = vec![0; CHUNK];
            let mut ordinal = 0_i64;
            let mut hasher = blake3::Hasher::new();
            let mut copied = 0_u64;
            loop {
                let count = input.read(&mut buffer).map_err(|e| e.to_string())?;
                if count == 0 {
                    break;
                }
                tx.execute(
                    "INSERT INTO editor_project_chunks VALUES(?1,?2,?3)",
                    params![relative, ordinal, &buffer[..count]],
                )
                .map_err(|e| e.to_string())?;
                hasher.update(&buffer[..count]);
                copied += count as u64;
                ordinal += 1;
            }
            if copied != size || hasher.finalize().to_hex().as_str() != digest {
                return Err("project resource changed while saving".into());
            }
        }
        verify_source_identity(&tx)?;
        tx.commit().map_err(|e| e.to_string())?;
        drop(output);
        fs::File::open(&staging)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())?;
        fs::rename(&staging, destination).map_err(|e| e.to_string())?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(staging);
    }
    result
}

fn collect(directory: &Path, files: &mut Vec<std::path::PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_symlink() {
            return Err("project resources cannot be symbolic links".into());
        }
        if kind.is_dir() {
            collect(&entry.path(), files)?;
        } else if kind.is_file() && !entry.file_name().to_string_lossy().ends_with(".part") {
            files.push(entry.path());
        }
    }
    Ok(())
}

pub(crate) fn open(project: &Path, root: &Path) -> Result<(), String> {
    if root.exists()
        && fs::read_dir(root)
            .map_err(|e| e.to_string())?
            .next()
            .is_some()
    {
        return Err("project working directory must be empty".into());
    }
    let input = readonly(project)?;
    verify(&input)?;
    fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let result = (|| -> Result<(), String> {
        let mut stmt = input
            .prepare("SELECT path,size,digest FROM editor_project_files ORDER BY path")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            let (path, size, digest) = row.map_err(|e| e.to_string())?;
            let size = u64::try_from(size).map_err(|e| e.to_string())?;
            let target = root.join(checked_path(&path)?);
            fs::create_dir_all(target.parent().ok_or("resource has no parent")?)
                .map_err(|e| e.to_string())?;
            let mut output = fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&target)
                .map_err(|e| e.to_string())?;
            let mut chunks = input
                .prepare(
                    "SELECT ordinal,data FROM editor_project_chunks WHERE path=?1 ORDER BY ordinal",
                )
                .map_err(|e| e.to_string())?;
            let mut rows = chunks.query([&path]).map_err(|e| e.to_string())?;
            let mut written = 0_u64;
            let mut expected = 0_i64;
            while let Some(row) = rows.next().map_err(|e| e.to_string())? {
                let ordinal: i64 = row.get(0).map_err(|e| e.to_string())?;
                let bytes: Vec<u8> = row.get(1).map_err(|e| e.to_string())?;
                if ordinal != expected || bytes.len() > CHUNK || written + bytes.len() as u64 > size
                {
                    return Err("invalid project resource chunks".into());
                }
                output.write_all(&bytes).map_err(|e| e.to_string())?;
                written += bytes.len() as u64;
                expected += 1;
            }
            output.sync_all().map_err(|e| e.to_string())?;
            if written != size
                || echo_core::hash_file(&target)
                    .map_err(|e| e.to_string())?
                    .to_string()
                    != digest
            {
                return Err("project resource integrity check failed".into());
            }
        }
        let database = root.join("catalog.sqlite");
        input
            .backup(rusqlite::MAIN_DB, &database, None)
            .map_err(|e| e.to_string())?;
        let output = Connection::open(database).map_err(|e| e.to_string())?;
        output.execute_batch("DROP TABLE editor_project_chunks; DROP TABLE editor_project_files; DELETE FROM catalog_meta WHERE key='editor_project_format'; VACUUM;").map_err(|e| e.to_string())?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(root);
    }
    result
}

#[cfg(test)]
mod tests;
