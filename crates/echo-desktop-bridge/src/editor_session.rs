//! Explicit source admission for a private editing session. No library jobs,
//! root scans, semantic indexing, or automatic inference are admitted here.
use crate::session::LibrarySession;
use echo_catalog::{AssetRegistrationInput, RegisterAsset};
use std::{fs, path::Path};

pub(crate) fn open(root: &Path) -> Result<LibrarySession, String> {
    let database = root.join("catalog.sqlite");
    if database.exists() {
        let connection = rusqlite::Connection::open_with_flags(
            &database,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .map_err(|e| e.to_string())?;
        verify_private(&connection)?;
    }
    let mut session = crate::session::open_session(
        database.to_str().ok_or("invalid project path")?,
        root.join("cache").to_str().ok_or("invalid cache path")?,
    )
    .map_err(|e| e.message)?;
    session.catalog.with_transaction(|tx| {
        tx.execute("INSERT OR REPLACE INTO catalog_meta(key,value) VALUES('session_kind','independent-editor-v1')", [])?;
        Ok::<_,echo_catalog::CatalogError>(())
    }).map_err(|e| e.to_string())?;
    session.independent = true;
    Ok(session)
}

pub(crate) fn verify_private(connection: &rusqlite::Connection) -> Result<(), String> {
    let independent: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM catalog_meta WHERE key='session_kind' AND value='independent-editor-v1')", [], |row| row.get(0)).map_err(|e| e.to_string())?;
    if !independent {
        return Err("this directory is not an independent editing session".into());
    }
    Ok(())
}

pub(crate) fn import(root: &Path, input: &Path) -> Result<String, String> {
    let session = open(root)?;
    let hash = echo_core::hash_file(input).map_err(|e| e.to_string())?;
    let name = input.file_name().ok_or("audio source has no filename")?;
    let relative = Path::new("media").join(hash.to_string()).join(name);
    let destination = root.join(&relative);
    fs::create_dir_all(destination.parent().ok_or("invalid media path")?)
        .map_err(|e| e.to_string())?;
    if !destination.exists() {
        let staging = destination.with_extension(format!("{}.part", uuid::Uuid::new_v4()));
        let result = (|| -> Result<(), String> {
            fs::copy(input, &staging).map_err(|e| e.to_string())?;
            if echo_core::hash_file(&staging).map_err(|e| e.to_string())? != hash {
                return Err("audio source changed while opening".into());
            }
            fs::File::open(&staging)
                .and_then(|f| f.sync_all())
                .map_err(|e| e.to_string())?;
            fs::rename(&staging, &destination).map_err(|e| e.to_string())
        })();
        if result.is_err() {
            let _ = fs::remove_file(staging);
        }
        result?;
    } else if echo_core::hash_file(&destination).map_err(|e| e.to_string())? != hash {
        return Err("project source integrity check failed".into());
    }
    let probe = echo_bridge::probe(&destination).map_err(|e| e.message)?;
    if !probe.has_audio || probe.duration_millis == 0 {
        return Err("file has no decodable audio".into());
    }
    let size = destination.metadata().map_err(|e| e.to_string())?.len();
    session
        .catalog
        .with_transaction(|tx| {
            let asset = match echo_catalog::register_asset(
                tx,
                &AssetRegistrationInput {
                    content_hash: hash,
                    path: Path::new(&relative.to_string_lossy().replace('\\', "/")),
                    size_bytes: size,
                    codec: Some(&probe.codec_name),
                    duration_millis: Some(probe.duration_millis),
                    recorded_at_millis: None,
                    imported_at_millis: crate::session::now_millis(),
                },
            )? {
                RegisterAsset::Created(a) | RegisterAsset::Existed(a) => a,
            };
            echo_catalog::set_sound_membership(tx, &asset.id.to_string(), false, false, "")?;
            echo_catalog::record_source_metadata(
                tx,
                asset.id,
                &echo_catalog::SourceMetadata {
                    container_format: probe.container_format,
                    sample_rate: probe.sample_rate,
                    channel_count: probe.channel_count,
                    entries: probe
                        .metadata
                        .into_iter()
                        .map(|e| echo_catalog::SourceMetadataEntry {
                            key: e.key,
                            value: e.value,
                        })
                        .collect(),
                },
                None,
            )?;
            Ok::<_, echo_catalog::CatalogError>(asset.id.to_string())
        })
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests;
