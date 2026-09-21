use std::path::Path;

use echo_catalog::{AssetRegistrationInput, RegisterAsset, register_asset};
use echo_domain::{AssetId, ContentHash, FadeCurve, SoundAssembly};

use crate::{ffi::RenderExportWire, session::open_session};

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
