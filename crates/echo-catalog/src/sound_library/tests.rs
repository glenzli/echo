use std::path::{Path, PathBuf};

use echo_domain::{
    AssemblyClip, AssemblyClipId, AssemblyMaster, AssemblyTrack, AssemblyTrackId, AssetId,
    ContentHash, FadeCurve, SoundAssembly, SoundAssemblyId,
};

use super::*;
use crate::{AssetRegistrationInput, Catalog, RegisterAsset, open_catalog, register_asset};

fn fixture() -> (PathBuf, Catalog, AssetId) {
    let root = std::env::temp_dir().join(format!("echo-memory-contract-{}", uuid::Uuid::now_v7()));
    let catalog = open_catalog(&root.join("catalog.sqlite")).unwrap();
    let id = catalog
        .with_transaction(|tx| {
            let result = register_asset(
                tx,
                &AssetRegistrationInput {
                    content_hash: ContentHash::new([31; 32]),
                    path: Path::new("/originals/shore.wav"),
                    size_bytes: 100,
                    codec: Some("pcm"),
                    duration_millis: Some(120_000),
                    recorded_at_millis: None,
                    imported_at_millis: 1,
                },
            )?;
            Ok::<_, CatalogError>(match result {
                RegisterAsset::Created(a) | RegisterAsset::Existed(a) => a.id,
            })
        })
        .unwrap();
    (root, catalog, id)
}

fn document(id: SoundAssemblyId, asset: AssetId, name: &str) -> SoundAssembly {
    SoundAssembly::new(
        id,
        name.to_owned(),
        AssemblyMaster::standard(),
        vec![
            AssemblyTrack::new(
                AssemblyTrackId::new(),
                "Shore".to_owned(),
                0,
                0,
                false,
                false,
                vec![
                    AssemblyClip::new(
                        AssemblyClipId::new(),
                        asset,
                        0,
                        0,
                        120_000,
                        0,
                        0,
                        0,
                        0,
                        0,
                        FadeCurve::Linear,
                        FadeCurve::Linear,
                        false,
                    )
                    .unwrap(),
                ],
            )
            .unwrap(),
        ],
    )
    .unwrap()
}

fn export(tx: &Transaction<'_>, doc: &SoundAssembly, millis: i64) -> Result<i64, CatalogError> {
    let revision = crate::record_sound_assembly(tx, doc, millis)?;
    crate::record_sound_assembly_export(
        tx,
        &crate::RecordSoundAssemblyExport {
            assembly_id: doc.id(),
            assembly_revision_id: revision.revision_id,
            output_path: Path::new("/managed/memories/shore-v1.wav"),
            format: crate::SoundAssemblyExportFormat::WavPcm24,
            sample_rate: 48_000,
            channel_count: 2,
            frame_count: 5_760_000,
            content_hash: ContentHash::new([42; 32]),
            size_bytes: 34_560_044,
            integrated_lufs: -18.0,
            true_peak_dbtp: -1.0,
            created_at_millis: millis,
        },
    )
    .map(|record| record.export_id)
}

#[test]
fn collection_membership_does_not_change_or_delete_originals() {
    let (root, catalog, id) = fixture();
    catalog
        .with_transaction(|tx| -> Result<_, CatalogError> {
            let before = crate::find_by_id(tx, id)?;
            set_sound_membership(tx, &id.to_string(), false, true, "ambience")?;
            assert_eq!(crate::find_by_id(tx, id)?, before);
            assert!(!sound_memberships(tx)?[&id.to_string()].in_memory);
            assert!(
                crate::revisit_snapshot(tx, 100)?
                    .recently_added_asset_ids
                    .is_empty()
            );
            set_sound_membership(tx, &id.to_string(), true, true, "ambience")?;
            assert!(sound_memberships(tx)?[&id.to_string()].in_materials);
            assert_eq!(
                crate::revisit_snapshot(tx, 100)?.recently_added_asset_ids,
                vec![id]
            );
            Ok(())
        })
        .unwrap();
    drop(catalog);
    let reopened = open_catalog(&root.join("catalog.sqlite")).unwrap();
    assert!(reopened.with_transaction(sound_memberships).unwrap()[&id.to_string()].in_materials);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn assembly_memory_uses_its_own_identity_and_pins_the_listening_edition() {
    let (root, catalog, id) = fixture();
    let assembly = document(SoundAssemblyId::new(), id, "The afternoon");
    let memory_id = assembly.id().to_string();
    catalog
        .with_transaction(|tx| -> Result<_, CatalogError> {
            let export_id = export(tx, &assembly, 10)?;
            preserve_assembly_memory(tx, &memory_id, export_id, 20)?;
            let item_id = memory_id.parse().unwrap();
            crate::set_asset_affinity(
                tx,
                item_id,
                crate::AssetAffinity {
                    liked: true,
                    rating: 5,
                },
                21,
            )?;
            crate::create_user_album(
                tx,
                crate::CreateUserAlbum {
                    name: "Summer",
                    member_asset_ids: &[item_id, id],
                },
                22,
            )?;
            crate::record_asset_listening_progress(tx, item_id, 40_000, 0, 120_000, 23)?;
            let updated = document(assembly.id(), id, "Changed draft");
            crate::record_sound_assembly(tx, &updated, 30)?;
            let memory = &assembly_memories(tx)?[0];
            assert_eq!(memory.name, "The afternoon");
            assert_eq!(memory.export_id, export_id);
            assert!(memory.liked);
            assert_eq!(memory.resume_position_millis, 40_000);
            assert_eq!(
                crate::list_assets(tx)?.len(),
                1,
                "a mix is never registered as an Original"
            );
            assert_eq!(crate::list_user_albums(tx)?[0].member_asset_ids.len(), 2);
            let new_export = export(tx, &updated, 31)?;
            preserve_assembly_memory(tx, &memory_id, new_export, 32)?;
            assert_eq!(assembly_memories(tx)?[0].name, "Changed draft");
            assert_eq!(assembly_memories(tx)?[0].resume_position_millis, 0);
            assert_eq!(
                tx.query_row::<i64, _, _>(
                    "SELECT COUNT(*) FROM assembly_memory_editions",
                    [],
                    |r| r.get(0)
                )?,
                2
            );
            Ok(())
        })
        .unwrap();
    drop(catalog);
    assert_eq!(
        open_catalog(&root.join("catalog.sqlite"))
            .unwrap()
            .with_transaction(assembly_memories)
            .unwrap()[0]
            .id,
        memory_id
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn cross_project_export_cannot_become_a_memory() {
    let (root, catalog, id) = fixture();
    let assembly = document(SoundAssemblyId::new(), id, "First");
    let result = catalog.with_transaction(|tx| -> Result<_, CatalogError> {
        let export_id = export(tx, &assembly, 10)?;
        preserve_assembly_memory(tx, &SoundAssemblyId::new().to_string(), export_id, 20)
    });
    assert!(result.is_err());
    assert!(
        catalog
            .with_transaction(assembly_memories)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        catalog.with_transaction(sound_memberships).unwrap().len(),
        1
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn previous_catalog_migrates_user_facts_once_without_replaying_them() {
    let (root, catalog, id) = fixture();
    catalog.with_transaction(|tx| -> Result<_,CatalogError> {
        tx.execute("INSERT INTO asset_user_state (asset_id, liked, rating, last_listened_at_millis, resume_position_millis, updated_at_millis) VALUES (?1,1,4,20,30000,20)",[id.to_string()])?;
        tx.execute("INSERT INTO user_albums VALUES (7,'Old album',?1,1,2)",[id.to_string()])?;
        tx.execute("INSERT INTO user_album_members VALUES (7,?1,2)",[id.to_string()])?;
        tx.execute_batch("DROP VIEW memory_sources; DROP TABLE memory_album_members; DROP TABLE memory_albums; DROP TABLE sound_user_state; DROP TABLE assembly_memory_editions; DROP TABLE project_materials; DROP TABLE project_adjustment_revisions; DROP TABLE memory_waveform_artifacts; DROP TABLE sound_items; UPDATE catalog_meta SET value='20260831.1' WHERE key='schema_version';")?;
        Ok(())
    }).unwrap();
    drop(catalog);
    let catalog = open_catalog(&root.join("catalog.sqlite")).unwrap();
    catalog
        .with_transaction(|tx| -> Result<_, CatalogError> {
            assert_eq!(crate::asset_affinity(tx, id)?.rating, 4);
            assert_eq!(
                crate::asset_listening_state(tx, id)?.resume_position_millis,
                30_000
            );
            assert_eq!(crate::list_user_albums(tx)?[0].id, 7);
            crate::delete_user_album(tx, 7)?;
            set_sound_membership(tx, &id.to_string(), false, true, "music")
        })
        .unwrap();
    drop(catalog);
    let catalog = open_catalog(&root.join("catalog.sqlite")).unwrap();
    assert!(
        catalog
            .with_transaction(crate::list_user_albums)
            .unwrap()
            .is_empty()
    );
    assert!(!catalog.with_transaction(sound_memberships).unwrap()[&id.to_string()].in_memory);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn current_catalog_missing_collection_tables_is_not_silently_recreated() {
    let (root, catalog, _) = fixture();
    catalog.with_transaction(|tx| -> Result<(), CatalogError> {
        tx.execute_batch("DROP VIEW memory_sources; DROP TABLE memory_album_members; DROP TABLE memory_albums; DROP TABLE sound_user_state; DROP TABLE assembly_memory_editions; DROP TABLE project_materials; DROP TABLE project_adjustment_revisions; DROP TABLE memory_waveform_artifacts; DROP TABLE sound_items;")?;
        Ok(())
    }).unwrap();
    drop(catalog);
    let error = open_catalog(&root.join("catalog.sqlite")).unwrap_err();
    assert_eq!(error.kind, CatalogErrorKind::SchemaMismatch);
    let connection = rusqlite::Connection::open(root.join("catalog.sqlite")).unwrap();
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name='sound_items'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
    drop(connection);
    std::fs::remove_dir_all(root).unwrap();
}
