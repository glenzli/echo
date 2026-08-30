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
    value["tracks"][0]["clips"][0]["fadeInCurve"] =
        serde_json::Value::String("equal_power".to_owned());
    let saved = session
        .save_sound_assembly(&serde_json::to_string(&value).expect("document encodes"))
        .expect("revision saves");
    assert_eq!(saved.revision_number, 2);
    let saved_document: SoundAssembly =
        serde_json::from_str(&saved.document_json).expect("saved document decodes");
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
