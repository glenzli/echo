use std::{fs, path::Path};

use echo_catalog::{AssetRegistrationInput, RegisterAsset, register_asset};
use echo_domain::ContentHash;

#[test]
fn session_records_verified_original_render() {
    let root = std::env::temp_dir().join(format!("echo-session-render-{}", std::process::id()));
    fs::create_dir_all(&root).expect("fixture root creates");
    let source = root.join("source.wav");
    let output = root.join("output.wav");
    fs::write(&source, b"immutable").expect("source writes");
    fs::write(&output, b"rendered").expect("output writes");
    let session = crate::session::open_session(
        root.join("catalog.sqlite").to_str().expect("utf8 path"),
        root.join("cache").to_str().expect("utf8 path"),
    )
    .expect("session opens");
    let asset = session
        .catalog()
        .with_transaction(|transaction| {
            let registered = register_asset(
                transaction,
                &AssetRegistrationInput {
                    content_hash: ContentHash::new([3; 32]),
                    path: Path::new(&source),
                    size_bytes: 9,
                    codec: Some("pcm"),
                    duration_millis: Some(1_000),
                    recorded_at_millis: None,
                    imported_at_millis: 1,
                },
            )?;
            Ok::<_, echo_catalog::CatalogError>(match registered {
                RegisterAsset::Created(asset) | RegisterAsset::Existed(asset) => asset,
            })
        })
        .expect("asset registers");
    let id = session
        .record_render_export(
            &asset.id.to_string(),
            0,
            output.to_str().expect("utf8 path"),
            "wav_pcm24",
            48_000,
            2,
            24,
            1,
            8,
            -18.0,
            -1.0,
            &session
                .export_source_disclosure(&asset.id.to_string(), 0)
                .unwrap(),
        )
        .expect("render records");
    assert!(id > 0);
    let captured = session
        .export_source_disclosure(&asset.id.to_string(), 0)
        .unwrap();
    session
        .set_source_disclosure(
            &asset.id.to_string(),
            0,
            r#"[{"kind":"ai_generated","startMillis":0,"endMillis":1000,"note":"Private note"}]"#,
        )
        .unwrap();
    assert!(
        session
            .record_render_export(
                &asset.id.to_string(),
                0,
                output.to_str().unwrap(),
                "wav_pcm24",
                48000,
                2,
                24,
                1,
                8,
                -18.0,
                -1.0,
                &captured
            )
            .is_err()
    );
    assert!(
        session
            .record_render_export(
                &asset.id.to_string(),
                0,
                output.to_str().unwrap(),
                "wav_pcm24",
                48000,
                2,
                24,
                1,
                8,
                -18.0,
                -1.0,
                ""
            )
            .is_err()
    );
    let count = session
        .catalog()
        .with_transaction(|tx| {
            Ok::<_, echo_catalog::CatalogError>(tx.query_row(
                "SELECT COUNT(*) FROM render_exports",
                [],
                |row| row.get::<_, i64>(0),
            )?)
        })
        .unwrap();
    assert_eq!(
        count, 1,
        "a changed declaration must not create a contradictory receipt"
    );
    let working_source = root.join("working.wav");
    let repaired_output = root.join("repaired-output.wav");
    fs::write(&working_source, b"frozen-render").expect("working source writes");
    fs::write(&repaired_output, b"repaired-delivery").expect("repaired output writes");
    let copy = session
        .create_rendered_spectral_working_copy(
            &asset.id.to_string(),
            0,
            working_source.to_str().expect("utf8 path"),
        )
        .expect("working copy records");
    let repaired_id = session
        .record_rendered_spectral_working_copy_export(
            &asset.id.to_string(),
            0,
            copy.record.id,
            copy.cache_path.to_str().expect("utf8 path"),
            repaired_output.to_str().expect("utf8 path"),
            "wav_pcm24",
            48_000,
            2,
            24,
            1,
            17,
            -18.0,
            -1.0,
            &session
                .export_source_disclosure(&asset.id.to_string(), 0)
                .unwrap(),
        )
        .expect("working-copy export records");
    let provenance_count: i64 = session
        .catalog()
        .with_transaction(|transaction| {
            Ok::<_, echo_catalog::CatalogError>(transaction.query_row(
                "SELECT COUNT(*) FROM render_export_working_copy_provenance \
                 WHERE render_export_id = ?1 AND working_copy_id = ?2",
                (repaired_id, copy.record.id),
                |row| row.get(0),
            )?)
        })
        .expect("working-copy provenance reads");
    assert_eq!(provenance_count, 1);
    let _ = fs::remove_dir_all(root);
}
