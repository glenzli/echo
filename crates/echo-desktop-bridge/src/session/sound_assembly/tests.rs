use std::path::Path;

use echo_catalog::{AssetRegistrationInput, RegisterAsset, register_asset};
use echo_domain::{AssetId, ContentHash, FadeCurve, SoundAssembly};

use crate::{ffi::RenderExportWire, session::open_session};

#[test]
fn historical_projection_keeps_exact_sources_and_does_not_publish() {
    let root = fixture_root("history-projection");
    let session = open_session(
        root.join("catalog.sqlite").to_str().unwrap(),
        root.join("cache").to_str().unwrap(),
    )
    .unwrap();
    let asset = register(&session.catalog, 0x76, "/sounds/history.wav", 2000);
    let first = session
        .create_sound_assembly("Before", &[asset.to_string()], 0)
        .unwrap();
    let mut changed: serde_json::Value = serde_json::from_str(&first.document_json).unwrap();
    changed["name"] = "After".into();
    changed["tracks"][0]["clips"][0]["gainCentibels"] = (-300).into();
    let latest = session.save_sound_assembly(&changed.to_string()).unwrap();
    let history = session
        .sound_assembly_history(&first.assembly_id, 0)
        .unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].revision_id, latest.revision_id);
    let old = session
        .sound_assembly_at_revision(&first.assembly_id, first.revision_id)
        .unwrap();
    assert_eq!(old.document_json, first.document_json);
    assert_eq!(old.clip_sources[0].path, first.clip_sources[0].path);
    assert_eq!(
        old.clip_sources[0].adjustment_revision_id,
        first.clip_sources[0].adjustment_revision_id
    );
    assert_eq!(
        session
            .sound_assembly(&first.assembly_id)
            .unwrap()
            .revision_id,
        latest.revision_id
    );
    let other = session
        .create_sound_assembly("Other", &[asset.to_string()], 0)
        .unwrap();
    assert!(
        session
            .sound_assembly_at_revision(&other.assembly_id, first.revision_id)
            .is_err()
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn library_selection_creates_and_reopens_a_revision_pinned_sequence() {
    let root = fixture_root("create");
    let session = open_session(
        root.join("catalog.sqlite").to_str().expect("catalog path"),
        root.join("cache").to_str().expect("cache path"),
    )
    .expect("session opens");
    let first = register(&session.catalog, 0x21, "/sounds/first.wav", 1_500);
    let second = register(&session.catalog, 0x22, "/sounds/second.wav", 2_000);
    session
        .catalog
        .with_transaction(|tx| {
            echo_catalog::set_sound_membership(tx, &second.to_string(), false, true, "ambience")
        })
        .unwrap();
    let created = session
        .create_sound_assembly(
            "Field sequence",
            &[first.to_string(), second.to_string()],
            0,
        )
        .expect("sequence creates");
    assert_eq!(created.revision_number, 1);
    assert_eq!(created.clip_sources.len(), 2);
    assert_eq!(created.clip_sources[0].adjustment_revision_id, 0);
    let document: SoundAssembly =
        serde_json::from_str(&created.document_json).expect("document decodes");
    assert_eq!(document.duration_millis(), 3_500);
    assert_eq!(document.tracks().len(), 1);
    assert_eq!(
        document.tracks()[0].clips()[0].source_role(),
        echo_domain::AssemblySourceRole::Memory
    );
    assert_eq!(
        document.tracks()[0].clips()[1].source_role(),
        echo_domain::AssemblySourceRole::Material
    );
    assert_eq!(
        document.tracks()[0].clips()[1].timeline_start_millis(),
        1_500
    );

    let reopened = session
        .sound_assembly(&created.assembly_id)
        .expect("sequence reopens");
    assert_eq!(reopened.revision_id, created.revision_id);
    assert_eq!(
        session.sound_assemblies().expect("assemblies list").len(),
        1
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn complete_document_save_and_verified_export_append_history() {
    let root = fixture_root("save-export");
    std::fs::create_dir_all(&root).expect("fixture root creates");
    let session = open_session(
        root.join("catalog.sqlite").to_str().expect("catalog path"),
        root.join("cache").to_str().expect("cache path"),
    )
    .expect("session opens");
    let asset = register(&session.catalog, 0x31, "/sounds/layer.wav", 1_000);
    let created = session
        .create_sound_assembly("Layer", &[asset.to_string()], 1)
        .expect("layer creates");
    let mut value: serde_json::Value =
        serde_json::from_str(&created.document_json).expect("document value decodes");
    value["name"] = serde_json::Value::String("Renamed layer".to_owned());
    value["markers"] = serde_json::json!([{"id": echo_domain::AssemblyMarkerId::new().to_string(), "name":"Opening", "startMillis":100, "endMillis":500}]);
    value["tracks"][0]["clips"][0]["fadeInCurve"] =
        serde_json::Value::String("equal_power".to_owned());
    let saved = session
        .save_sound_assembly(&serde_json::to_string(&value).expect("document encodes"))
        .expect("revision saves");
    assert_eq!(saved.revision_number, 2);
    let saved_document: SoundAssembly =
        serde_json::from_str(&saved.document_json).expect("saved document decodes");
    assert_eq!(saved_document.markers()[0].name(), "Opening");
    assert_eq!(saved_document.markers()[0].end_millis(), Some(500));
    let reopened = session.sound_assembly(&saved.assembly_id).unwrap();
    assert_eq!(reopened.document_json, saved.document_json);
    assert_eq!(
        saved_document.tracks()[0].clips()[0].fade_in_curve(),
        FadeCurve::EqualPower
    );

    let output = root.join("layer-mix.wav");
    std::fs::write(&output, b"verified assembly bytes").expect("mixdown fixture writes");
    let export_id = session
        .record_sound_assembly_export(
            &saved.assembly_id,
            saved.revision_id,
            &RenderExportWire {
                source_disclosure_comment: session
                    .export_source_disclosure(&saved.assembly_id, saved.revision_id)
                    .unwrap(),
                output_path: output.to_string_lossy().into_owned(),
                format: "wav_pcm24".to_owned(),
                sample_rate: 48_000,
                channel_count: 2,
                bit_depth: 24,
                frame_count: 48_000,
                size_bytes: 23,
                integrated_lufs: -18.0,
                true_peak_dbtp: -1.0,
            },
        )
        .expect("export records");
    assert!(export_id > 0);
    session
        .archive_sound_assembly(&saved.assembly_id)
        .expect("assembly archives");
    assert!(
        session
            .sound_assemblies()
            .expect("assemblies list")
            .is_empty()
    );
    let _ = std::fs::remove_dir_all(root);
}

fn register(
    catalog: &echo_catalog::Catalog,
    byte: u8,
    path: &str,
    duration_millis: u64,
) -> AssetId {
    let registered = catalog
        .with_transaction(|transaction| {
            register_asset(
                transaction,
                &AssetRegistrationInput {
                    content_hash: ContentHash::new([byte; 32]),
                    path: Path::new(path),
                    size_bytes: 512,
                    codec: Some("wav"),
                    duration_millis: Some(duration_millis),
                    recorded_at_millis: None,
                    imported_at_millis: 1,
                },
            )
        })
        .expect("asset registers");
    match registered {
        RegisterAsset::Created(asset) | RegisterAsset::Existed(asset) => asset.id,
    }
}

fn fixture_root(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "echo-desktop-assembly-{label}-{}-{}",
        std::process::id(),
        uuid::Uuid::now_v7(),
    ))
}

#[test]
fn clip_processing_is_isolated_and_reopens_the_exact_revision() {
    let root = fixture_root("clip-scope");
    let session = open_session(
        root.join("catalog.sqlite").to_str().unwrap(),
        root.join("cache").to_str().unwrap(),
    )
    .unwrap();
    let asset = register(&session.catalog, 0x74, "/sounds/source.wav", 2000);
    let created = session
        .create_sound_assembly("A memory", &[asset.to_string()], 0)
        .unwrap();
    let source = &created.clip_sources[0];
    let fields = crate::session::adjustment_wire_fields(None, Some(2000));
    let mut adjustment = crate::session::asset_adjustment_wire(fields);
    adjustment.gain_centibels = -600;
    let saved = session
        .save_project_clip_adjustment(
            &created.document_json,
            &source.clip_id,
            &asset.to_string(),
            &adjustment,
        )
        .unwrap();
    assert_eq!(saved.clip_sources[0].adjustment.gain_centibels, -600);
    assert!(saved.clip_sources[0].adjustment_revision_id > 0);
    let original = session.list_assets().unwrap().remove(0);
    assert_eq!(original.gain_centibels, 0);
    assert_eq!(original.adjustment_revision, 0);
    let reopened = session.sound_assembly(&created.assembly_id).unwrap();
    assert_eq!(
        reopened.clip_sources[0].adjustment_revision_id,
        saved.clip_sources[0].adjustment_revision_id
    );
    let invalid = session.save_project_clip_adjustment(
        &saved.document_json,
        "deleted-clip",
        &asset.to_string(),
        &adjustment,
    );
    assert!(invalid.is_err());
    assert_eq!(
        session
            .sound_assembly(&created.assembly_id)
            .unwrap()
            .revision_id,
        saved.revision_id
    );
    drop(session);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn precision_source_trim_reanchors_bypassed_assembly_gain() {
    let root = fixture_root("envelope-source-edit");
    let session = open_session(
        root.join("catalog.sqlite").to_str().unwrap(),
        root.join("cache").to_str().unwrap(),
    )
    .unwrap();
    let asset = register(&session.catalog, 0x75, "/sounds/source.wav", 4000);
    let created = session
        .create_sound_assembly("Trimmed speech", &[asset.to_string()], 0)
        .unwrap();
    let mut document: serde_json::Value = serde_json::from_str(&created.document_json).unwrap();
    document["tracks"][0]["clips"][0]["gainEnvelope"] = serde_json::json!({"enabled":false,"points":[
        {"sourceMillis":0,"gainCentibels":0}, {"sourceMillis":4000,"gainCentibels":-4000}
    ]});
    let mut adjustment = crate::session::asset_adjustment_wire(
        crate::session::adjustment_wire_fields(None, Some(4000)),
    );
    adjustment.trim_start_millis = 1000;
    adjustment.trim_end_millis = 3500;
    adjustment.edit_segments.clear();
    let saved = session
        .save_project_clip_adjustment(
            &document.to_string(),
            &created.clip_sources[0].clip_id,
            &asset.to_string(),
            &adjustment,
        )
        .unwrap();
    let authored: SoundAssembly = serde_json::from_str(&saved.document_json).unwrap();
    let envelope: echo_domain::GainEnvelope = serde_json::from_value(serde_json::from_str::<serde_json::Value>(&saved.document_json).unwrap()["tracks"][0]["clips"][0]["gainEnvelope"].clone()).unwrap();
    assert!(!envelope.enabled);
    assert_eq!(envelope.points[0].source_millis, 0);
    assert_eq!(envelope.points[0].gain_centibels, -1000);
    assert_eq!(authored.duration_millis(), 2500);
    assert_eq!(
        session
            .sound_assembly(&created.assembly_id)
            .unwrap()
            .document_json,
        saved.document_json
    );
    drop(session);
    std::fs::remove_dir_all(root).unwrap();
}

fn based_document(revision: &crate::ffi::SoundAssemblyRevisionWire) -> serde_json::Value {
    let mut document: serde_json::Value = serde_json::from_str(&revision.document_json).unwrap();
    document["revisionId"] = revision.revision_id.into();
    document
}

#[test]
fn assembly_save_checks_the_base_before_deduplication_and_can_retry_after_reload() {
    let root = fixture_root("expected-base");
    let catalog = root.join("catalog.sqlite");
    let cache = root.join("cache");
    let first = open_session(catalog.to_str().unwrap(), cache.to_str().unwrap()).unwrap();
    let second = open_session(catalog.to_str().unwrap(), cache.to_str().unwrap()).unwrap();
    let asset = register(&first.catalog, 0x78, "/sounds/base.wav", 2000);
    let initial = first
        .create_sound_assembly("Initial", &[asset.to_string()], 0)
        .unwrap();
    let mut stale = based_document(&initial);
    let mut remote = stale.clone();
    remote["name"] = "Elsewhere".into();
    let latest = second.save_sound_assembly(&remote.to_string()).unwrap();
    for name in ["Unsaved draft", "Elsewhere"] {
        stale["name"] = name.into();
        let error = first.save_sound_assembly(&stale.to_string()).err().unwrap();
        assert!(
            error
                .to_string()
                .starts_with("[assembly_revision_conflict]")
        );
        assert_eq!(
            first
                .sound_assembly(&initial.assembly_id)
                .unwrap()
                .revision_id,
            latest.revision_id
        );
        assert_eq!(
            first
                .sound_assembly_history(&initial.assembly_id, 0)
                .unwrap()
                .len(),
            2
        );
    }
    let current = based_document(&latest);
    assert_eq!(
        first
            .save_sound_assembly(&current.to_string())
            .unwrap()
            .revision_id,
        latest.revision_id
    );
    stale["revisionId"] = latest.revision_id.into();
    stale["name"] = "Explicitly retried draft".into();
    let retried = first.save_sound_assembly(&stale.to_string()).unwrap();
    assert_eq!(retried.revision_number, 3);
    assert!(
        !serde_json::from_str::<serde_json::Value>(&retried.document_json)
            .unwrap()
            .as_object()
            .unwrap()
            .contains_key("revisionId")
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn assembly_save_rejects_invalid_or_foreign_bases_without_publishing() {
    let root = fixture_root("invalid-base");
    let session = open_session(
        root.join("catalog.sqlite").to_str().unwrap(),
        root.join("cache").to_str().unwrap(),
    )
    .unwrap();
    let asset = register(&session.catalog, 0x79, "/sounds/base.wav", 2000);
    let initial = session
        .create_sound_assembly("Initial", &[asset.to_string()], 0)
        .unwrap();
    let other = session
        .create_sound_assembly("Other", &[asset.to_string()], 0)
        .unwrap();
    let mut document = based_document(&initial);
    document["name"] = "Must not publish".into();
    for expected in serde_json::json!([null, "1", -1, 1.5, true, 18446744073709551615u64])
        .as_array()
        .unwrap()
    {
        document["revisionId"] = expected.clone();
        assert!(
            session
                .save_sound_assembly(&document.to_string())
                .err()
                .unwrap()
                .to_string()
                .contains("invalid expected assembly revision")
        );
    }
    for expected in [0, other.revision_id] {
        document["revisionId"] = expected.into();
        assert!(
            session
                .save_sound_assembly(&document.to_string())
                .err()
                .unwrap()
                .to_string()
                .starts_with("[assembly_revision_conflict]")
        );
    }
    assert_eq!(
        session
            .sound_assembly_history(&initial.assembly_id, 0)
            .unwrap()
            .len(),
        1
    );
    // Existing internal callers without a base retain their legacy contract.
    document.as_object_mut().unwrap().remove("revisionId");
    assert_eq!(
        session
            .save_sound_assembly(&document.to_string())
            .unwrap()
            .revision_number,
        2
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn stale_clip_save_does_not_append_adjustments_and_fresh_retry_is_atomic() {
    let root = fixture_root("clip-expected-base");
    let catalog = root.join("catalog.sqlite");
    let cache = root.join("cache");
    let first = open_session(catalog.to_str().unwrap(), cache.to_str().unwrap()).unwrap();
    let second = open_session(catalog.to_str().unwrap(), cache.to_str().unwrap()).unwrap();
    let asset = register(&first.catalog, 0x7a, "/sounds/clip.wav", 2000);
    let initial = first
        .create_sound_assembly("Initial", &[asset.to_string()], 0)
        .unwrap();
    let stale = based_document(&initial);
    let mut remote = stale.clone();
    remote["name"] = "Elsewhere".into();
    let latest = second.save_sound_assembly(&remote.to_string()).unwrap();
    let fields = crate::session::adjustment_wire_fields(None, Some(2000));
    let mut adjustment = crate::session::asset_adjustment_wire(fields);
    adjustment.gain_centibels = -600;
    let counts = || {
        first
            .catalog
            .with_transaction::<_, echo_catalog::CatalogError>(|tx| {
                Ok((
                    tx.query_row(
                        "SELECT COUNT(*) FROM asset_adjustment_revisions",
                        [],
                        |row| row.get::<_, i64>(0),
                    )
                    .unwrap(),
                    tx.query_row(
                        "SELECT COUNT(*) FROM project_adjustment_revisions",
                        [],
                        |row| row.get::<_, i64>(0),
                    )
                    .unwrap(),
                ))
            })
            .unwrap()
    };
    let before = counts();
    let clip = &initial.clip_sources[0].clip_id;
    let error = first
        .save_project_clip_adjustment(&stale.to_string(), clip, &asset.to_string(), &adjustment)
        .err()
        .unwrap();
    assert!(
        error
            .to_string()
            .starts_with("[assembly_revision_conflict]")
    );
    assert_eq!(counts(), before);
    assert_eq!(
        first
            .sound_assembly(&initial.assembly_id)
            .unwrap()
            .revision_id,
        latest.revision_id
    );
    let mut invalid = based_document(&latest);
    invalid["tracks"][0]["gainCentibels"] = 100_000.into();
    assert!(
        first
            .save_project_clip_adjustment(
                &invalid.to_string(),
                clip,
                &asset.to_string(),
                &adjustment
            )
            .is_err()
    );
    assert_eq!(
        counts(),
        before,
        "invalid authored content must roll back the appended adjustment"
    );
    assert_eq!(
        first
            .sound_assembly(&initial.assembly_id)
            .unwrap()
            .revision_id,
        latest.revision_id
    );
    let saved = first
        .save_project_clip_adjustment(
            &based_document(&latest).to_string(),
            clip,
            &asset.to_string(),
            &adjustment,
        )
        .unwrap();
    assert_eq!(saved.revision_number, 3);
    assert_eq!(counts(), (before.0 + 1, before.1 + 1));
    assert!(saved.clip_sources[0].adjustment_revision_id > 0);
    first
        .catalog
        .with_transaction::<_, echo_catalog::CatalogError>(|tx| {
            assert!(echo_catalog::latest_adjustment_graph(tx, asset)?.is_none());
            Ok(())
        })
        .unwrap();
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn competing_sessions_cannot_both_publish_from_the_same_assembly_base() {
    let root = fixture_root("competing-bases");
    let catalog = root.join("catalog.sqlite");
    let cache = root.join("cache");
    let session = open_session(catalog.to_str().unwrap(), cache.to_str().unwrap()).unwrap();
    let asset = register(&session.catalog, 0x7b, "/sounds/concurrent.wav", 2000);
    let initial = session
        .create_sound_assembly("Initial", &[asset.to_string()], 0)
        .unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let mut handles = Vec::new();
    for name in ["First writer", "Second writer"] {
        let writer = open_session(catalog.to_str().unwrap(), cache.to_str().unwrap()).unwrap();
        let mut document = based_document(&initial);
        document["name"] = name.into();
        let barrier = barrier.clone();
        handles.push(std::thread::spawn(move || {
            barrier.wait();
            writer.save_sound_assembly(&document.to_string()).is_ok()
        }));
    }
    let successes = handles
        .into_iter()
        .map(|handle| usize::from(handle.join().unwrap()))
        .sum::<usize>();
    assert_eq!(successes, 1);
    assert_eq!(
        session
            .sound_assembly_history(&initial.assembly_id, 0)
            .unwrap()
            .len(),
        2
    );
    let stale = based_document(&initial);
    assert!(
        session
            .save_sound_assembly(&stale.to_string())
            .err()
            .unwrap()
            .to_string()
            .starts_with("[assembly_revision_conflict]")
    );
    let _ = std::fs::remove_dir_all(root);
}
