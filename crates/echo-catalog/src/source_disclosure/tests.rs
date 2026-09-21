use super::*;
use crate::*;
use echo_domain::*;
use std::path::Path;

fn register(tx: &Transaction<'_>, value: u8) -> AssetId {
    match register_asset(
        tx,
        &AssetRegistrationInput {
            content_hash: ContentHash::new([value; 32]),
            path: Path::new("/source.wav"),
            size_bytes: 100,
            codec: Some("pcm"),
            duration_millis: Some(1000),
            recorded_at_millis: None,
            imported_at_millis: 1,
        },
    )
    .unwrap()
    {
        RegisterAsset::Created(a) | RegisterAsset::Existed(a) => a.id,
    }
}
fn span() -> SourceDisclosureSpan {
    SourceDisclosureSpan {
        kind: SourceDisclosureKind::AiGenerated,
        start_millis: 200,
        end_millis: 400,
        note: "Added rain".into(),
    }
}
fn track(asset: AssetId, muted: bool, solo: bool, clip_muted: bool) -> AssemblyTrack {
    let clip = AssemblyClip::new(
        AssemblyClipId::new(),
        asset,
        0,
        0,
        1000,
        0,
        0,
        0,
        0,
        0,
        FadeCurve::Linear,
        FadeCurve::Linear,
        clip_muted,
    )
    .unwrap();
    AssemblyTrack::new(
        AssemblyTrackId::new(),
        "Source".into(),
        0,
        0,
        muted,
        solo,
        vec![clip],
    )
    .unwrap()
}
fn document(id: SoundAssemblyId, tracks: Vec<AssemblyTrack>) -> SoundAssembly {
    SoundAssembly::new(id, "Memory".into(), AssemblyMaster::standard(), tracks).unwrap()
}

#[test]
fn revision_guards_corrections_and_export_snapshots_survive_reopen() {
    let root = std::env::temp_dir().join(format!("echo-disclosure-{}", uuid::Uuid::now_v7()));
    let path = root.join("catalog.sqlite");
    let catalog = open_catalog(&path).unwrap();
    catalog
        .with_transaction(|tx| -> Result<(), CatalogError> {
            let id = register(tx, 1);
            assert!(
                !list_audio_space(tx)?[0]
                    .source_disclosure
                    .has_generated_source()
            );
            let rev = record_source_disclosure(tx, id, 0, &[span()], 2)?;
            assert_eq!(record_source_disclosure(tx, id, rev, &[span()], 3)?, rev);
            assert!(record_source_disclosure(tx, id, 0, &[], 3).is_err());
            let mut invalid = span();
            invalid.end_millis = 1001;
            assert!(record_source_disclosure(tx, id, rev, &[invalid], 3).is_err());
            assert!(
                list_audio_space(tx)?[0]
                    .source_disclosure
                    .has_generated_source()
            );
            let evidence = RecordRenderExport {
                asset_id: id,
                adjustment_revision_id: 0,
                output_path: "/export.wav".into(),
                format: RenderExportFormat::WavPcm24,
                sample_rate: 48000,
                channel_count: 1,
                bit_depth: 24,
                frame_count: 48000,
                content_hash: ContentHash::new([9; 32]),
                size_bytes: 144044,
                integrated_lufs: -18.0,
                true_peak_dbtp: -1.0,
                created_at_millis: 4,
            };
            let export = record_render_export(tx, &evidence)?;
            record_source_disclosure(tx, id, rev, &[], 5)?;
            assert!(
                !list_audio_space(tx)?[0]
                    .source_disclosure
                    .has_generated_source()
            );
            let corrected = record_render_export(tx, &evidence)?;
            assert_ne!(corrected.id, export.id);
            assert_eq!(record_render_export(tx, &evidence)?.id, corrected.id);
            let current: String = tx.query_row(
                "SELECT disclosure_json FROM render_export_disclosures WHERE render_export_id=?1",
                [corrected.id],
                |r| r.get(0),
            )?;
            assert!(
                !serde_json::from_str::<SourceDisclosureSummary>(&current)
                    .unwrap()
                    .has_generated_source()
            );
            let frozen: String = tx.query_row(
                "SELECT disclosure_json FROM render_export_disclosures WHERE render_export_id=?1",
                [export.id],
                |r| r.get(0),
            )?;
            assert!(
                serde_json::from_str::<SourceDisclosureSummary>(&frozen)
                    .unwrap()
                    .has_generated_source()
            );
            assert_eq!(
                tx.query_row("SELECT COUNT(*) FROM asset_source_disclosures", [], |r| r
                    .get::<_, i64>(
                    0
                ))?,
                2
            );
            Ok(())
        })
        .unwrap();
    drop(catalog);
    let catalog = open_catalog(&path).unwrap();
    assert_eq!(
        catalog
            .with_transaction(source_disclosures)
            .unwrap()
            .values()
            .next()
            .unwrap()
            .spans
            .len(),
        0
    );
    drop(catalog);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn retained_mix_uses_published_revision_and_audible_references() {
    let root = std::env::temp_dir().join(format!("echo-mix-disclosure-{}", uuid::Uuid::now_v7()));
    let catalog = open_catalog(&root.join("catalog.sqlite")).unwrap();
    catalog.with_transaction(|tx| -> Result<(),CatalogError> {
        let generated=register(tx,2); let unknown=register(tx,3);
        record_source_disclosure(tx,generated,0,&[span()],2)?;
        let all=source_disclosures(tx)?; let id=SoundAssemblyId::new();
        for (muted,solo,clip_muted,expected) in [(false,false,false,true),(true,false,false,false),(false,true,false,false),(false,false,true,false)] {
            let assembly=document(id,vec![track(generated,muted,false,clip_muted),track(unknown,false,solo,false)]);
            assert_eq!(assembly_source_disclosure(&assembly,&all).has_generated_source(),expected);
        }
        let mixed=document(id,vec![track(generated,false,false,false),track(unknown,false,false,false)]);
        let revision=record_sound_assembly(tx,&mixed,3)?;
        let export=record_sound_assembly_export(tx,&RecordSoundAssemblyExport { format:SoundAssemblyExportFormat::WavPcm24, assembly_id:id, assembly_revision_id:revision.revision_id, output_path:Path::new("/mix.wav"), sample_rate:48000,channel_count:2,frame_count:48000,content_hash:ContentHash::new([8;32]),size_bytes:288044,integrated_lufs:-18.0,true_peak_dbtp:-1.0,created_at_millis:4 })?;
        preserve_assembly_memory(tx,&id.to_string(),export.export_id,5)?;
        record_sound_assembly(tx,&document(id,vec![track(unknown,false,false,false)]),6)?;
        let memory=list_audio_space(tx)?.into_iter().find(|a|a.id==id.to_string()).unwrap();
        assert!(memory.source_disclosure.has_generated_source());
        assert_eq!(memory.assembly_revision_id,revision.revision_id);
        assert!(serde_json::from_str::<serde_json::Value>(&memory.provenance_json).unwrap()["sourceDisclosure"]["sources"].as_array().unwrap().len()==1);
        Ok(())
    }).unwrap();
    drop(catalog);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn preceding_schema_adds_empty_unknown_disclosures_without_relabeling() {
    let root =
        std::env::temp_dir().join(format!("echo-disclosure-upgrade-{}", uuid::Uuid::now_v7()));
    let path = root.join("catalog.sqlite");
    let catalog = open_catalog(&path).unwrap();
    catalog.with_transaction(|tx| -> Result<(),CatalogError> {
        register(tx,4);
        tx.execute_batch("DROP TABLE render_export_disclosures; DROP TABLE asset_source_disclosures; UPDATE catalog_meta SET value='20260920.2' WHERE key='schema_version';")?; Ok(())
    }).unwrap();
    drop(catalog);
    let catalog = open_catalog(&path).unwrap();
    let assets = catalog.with_transaction(list_audio_space).unwrap();
    assert_eq!(assets.len(), 1);
    assert!(assets[0].source_disclosure.sources.is_empty());
    drop(catalog);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn imported_declarations_are_conservative_and_manual_clears_survive_refresh() {
    let root =
        std::env::temp_dir().join(format!("echo-imported-disclosure-{}", uuid::Uuid::now_v7()));
    let catalog = open_catalog(&root.join("catalog.sqlite")).unwrap();
    catalog
        .with_transaction(|tx| -> Result<(), CatalogError> {
            let id = register(tx, 5);
            let comment = encode_portable_disclosure([
                SourceDisclosureKind::AiGenerated,
                SourceDisclosureKind::AiProcessed,
            ]);
            let metadata = SourceMetadata {
                container_format: "wav".into(),
                sample_rate: 48000,
                channel_count: 2,
                entries: vec![SourceMetadataEntry {
                    key: "comment".into(),
                    value: comment.clone(),
                }],
            };
            record_source_metadata(tx, id, &metadata, None)?;
            let summary = asset_source_disclosure(tx, id)?;
            assert!(summary.has_generated_source());
            assert!(summary.has_ai_processed_source());
            assert_eq!(summary.portable_comment(), comment);
            let imported = &summary.sources[0];
            assert_eq!(imported.origin, SourceDisclosureOrigin::EmbeddedExport);
            assert_eq!(imported.revision_id, 0);
            assert!(
                imported
                    .spans
                    .iter()
                    .all(|s| s.start_millis == 0 && s.end_millis == 1000 && s.note.is_empty())
            );
            let cleared = record_source_disclosure(tx, id, 0, &[], 2)?;
            assert!(cleared > 0);
            record_source_metadata(tx, id, &metadata, None)?;
            assert!(!asset_source_disclosure(tx, id)?.has_generated_source());
            assert!(record_source_disclosure(tx, id, 0, &[span()], 3).is_err());
            record_source_disclosure(tx, id, cleared, &[span()], 3)?;
            let summary = asset_source_disclosure(tx, id)?;
            assert_eq!(
                summary.sources[0].origin,
                SourceDisclosureOrigin::UserDeclared
            );
            assert!(!summary.portable_comment().contains("Added rain"));
            assert!(!summary.portable_comment().contains(&id.to_string()));
            let other = register(tx, 6);
            let foreign = SourceMetadata {
                entries: vec![
                    SourceMetadataEntry {
                        key: "title".into(),
                        value: comment.clone(),
                    },
                    SourceMetadataEntry {
                        key: "comment".into(),
                        value: comment.replace(".v1", ".v2"),
                    },
                ],
                ..metadata
            };
            record_source_metadata(tx, other, &foreign, None)?;
            assert!(asset_source_disclosure(tx, other)?.sources.is_empty());
            Ok(())
        })
        .unwrap();
    drop(catalog);
    std::fs::remove_dir_all(root).unwrap();
}
