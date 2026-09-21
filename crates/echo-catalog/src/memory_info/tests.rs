use super::*;
use crate::*;
use echo_domain::*;

#[test]
fn memory_info_migration_reopen_conflict_clear_and_assembly_separation() {
    let root = std::env::temp_dir().join(format!("echo-memory-{}", uuid::Uuid::now_v7()));
    let path = root.join("catalog.sqlite");
    let catalog = open_catalog(&path).unwrap();
    let (id, assembly_id) = catalog.with_transaction(|tx| {
        let asset = register_asset(tx, &AssetRegistrationInput { content_hash: ContentHash::new([21;32]), path: std::path::Path::new("/original.wav"), size_bytes: 100, codec: Some("pcm"), duration_millis: Some(1000), recorded_at_millis: Some(50), imported_at_millis: 100 })?;
        let id = match asset { RegisterAsset::Created(a) | RegisterAsset::Existed(a) => a.id.to_string() };
        let assembly = SoundAssembly::new(SoundAssemblyId::new(), "Travel".into(), AssemblyMaster::standard(), vec![AssemblyTrack::new(AssemblyTrackId::new(), "Source".into(), 0, 0, false, false, vec![AssemblyClip::new(AssemblyClipId::new(), id.parse().unwrap(), 0, 0, 1000, 0, 0, 0, 0, 0, FadeCurve::Linear, FadeCurve::Linear, false).unwrap()]).unwrap()]).unwrap();
        record_sound_assembly(tx, &assembly, 101)?;
        // Simulate the actual preceding schema, retaining Original and assembly state.
        tx.execute_batch("DROP TABLE memory_info_revisions; UPDATE catalog_meta SET value='20260922.3' WHERE key='schema_version';")?;
        Ok::<_, CatalogError>((id, assembly.id().to_string()))
    }).unwrap();
    drop(catalog);
    let catalog = open_catalog(&path).unwrap();
    let info = MemoryInfo {
        notes: "Personal note".into(),
        place: "Grandma’s balcony".into(),
        time_description: "Summer, roughly 2020".into(),
        moments: vec![MemoryMoment {
            start_millis: 10,
            end_millis: Some(900),
            note: "A laugh".into(),
        }],
    };
    let revision = catalog.with_transaction(|tx| {
        assert!(memory_info(tx, "missing", false).is_err());
        let revision = record_memory_info(tx, &id, false, 0, info.clone(), 102)?;
        assert_eq!(record_memory_info(tx, &id, false, revision, info.clone(), 103)?, revision);
        assert!(record_memory_info(tx, &id, false, 0, MemoryInfo::default(), 104).is_err());
        assert_eq!(memory_info(tx, &assembly_id, true)?.info, MemoryInfo::default());
        assert!(record_memory_info(tx, &assembly_id, true, 0, info.clone(), 104).is_err());
        record_memory_info(tx, &assembly_id, true, 0, MemoryInfo { notes: "Whole trip".into(), ..MemoryInfo::default() }, 104)?;
        let original = find_by_id(tx, id.parse().unwrap())?;
        assert!(matches!(original, AssetLookup::Found(a) if a.original.recorded_at_millis == Some(50)));
        Ok::<_, CatalogError>(revision)
    }).unwrap();
    drop(catalog);
    let catalog = open_catalog(&path).unwrap();
    catalog
        .with_transaction(|tx| {
            assert_eq!(memory_info(tx, &id, false)?.info, info);
            let cleared = record_memory_info(tx, &id, false, revision, MemoryInfo::default(), 105)?;
            assert!(cleared > revision);
            assert_eq!(memory_info(tx, &id, false)?.info, MemoryInfo::default());
            assert_eq!(
                memory_info(tx, &assembly_id, true)?.info.notes,
                "Whole trip"
            );
            let count: i64 = tx.query_row(
                "SELECT count(*) FROM memory_info_revisions WHERE asset_id=?1",
                [&id],
                |r| r.get(0),
            )?;
            assert_eq!(count, 2);
            Ok::<_, CatalogError>(())
        })
        .unwrap();
    drop(catalog);
    std::fs::remove_dir_all(root).unwrap();
}
