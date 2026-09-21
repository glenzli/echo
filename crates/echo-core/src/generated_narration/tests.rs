use super::*;
use echo_catalog::{CatalogError, SourceDisclosureOrigin};

fn fixture() -> (PathBuf, NarrationCandidate) {
    let root = std::env::temp_dir().join(format!("echo-narration-{}", echo_domain::AssetId::new()));
    fs::create_dir_all(&root).unwrap();
    let frames = 8000u32;
    let mut wav = Vec::new();
    wav.extend(b"RIFF");
    wav.extend((36 + frames * 2).to_le_bytes());
    wav.extend(b"WAVEfmt ");
    wav.extend(16u32.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(8000u32.to_le_bytes());
    wav.extend(16000u32.to_le_bytes());
    wav.extend(2u16.to_le_bytes());
    wav.extend(16u16.to_le_bytes());
    wav.extend(b"data");
    wav.extend((frames * 2).to_le_bytes());
    wav.resize(44 + frames as usize * 2, 0);
    let runtime = serde_json::from_value(serde_json::json!({"contract_version":"test", "job":{
        "id":"test-generation", "consumer_core_contract":"test", "app_id":"echo", "intent":"speech.synthesize",
        "provider":"test","deployment":"test","model_profile":"test","model_build":"fixed-build","physical_model":"test-tts",
        "placement":"local","state":"succeeded","policy":"local-first","priority":"background"
    }})).unwrap();
    let candidate = stage("A remembered afternoon.", &root, &wav, runtime).unwrap();
    (root, candidate)
}

#[test]
fn acceptance_is_explicit_durable_idempotent_and_never_analysis() {
    let (root, candidate) = fixture();
    let catalog = echo_catalog::open_catalog(&root.join("project/catalog.sqlite")).unwrap();
    catalog
        .with_transaction(|tx| -> Result<(), CatalogError> {
            assert!(echo_catalog::list_assets(tx)?.is_empty());
            tx.execute(
                "INSERT INTO catalog_meta VALUES('session_kind','independent-editor-v1')",
                [],
            )?;
            Ok(())
        })
        .unwrap();
    let id = accept_narration(&catalog, &candidate, "", true).unwrap();
    assert_eq!(
        accept_narration(&catalog, &candidate, "", true).unwrap(),
        id
    );
    fs::remove_file(&candidate.path).unwrap();
    catalog
        .with_transaction(|tx| -> Result<(), CatalogError> {
            let asset = &echo_catalog::list_assets(tx)?[0];
            assert!(asset.original.path.starts_with("media/generated"));
            let durable = root.join("project").join(&asset.original.path);
            assert!(durable.exists());
            assert_eq!(
                crate::hash_file(&durable).unwrap().to_string(),
                candidate.receipt.output_hash
            );
            let membership = &echo_catalog::sound_memberships(tx)?[&id];
            assert!(!membership.in_memory && !membership.in_materials);
            let summary = echo_catalog::asset_source_disclosure(tx, asset.id)?;
            assert!(summary.has_generated_source());
            assert!(summary.portable_comment().contains("ai_generated"));
            assert_eq!(
                summary.sources[0].origin,
                SourceDisclosureOrigin::RuntimeGenerated
            );
            assert_eq!(
                summary.sources[0].generation.as_ref().unwrap()["input_text"],
                "A remembered afternoon."
            );
            assert!(echo_catalog::record_source_disclosure(tx, asset.id, 0, &[], 10).is_err());
            for table in ["jobs", "analysis_records"] {
                let count: i64 =
                    tx.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))?;
                assert_eq!(count, 0);
            }
            let count: i64 =
                tx.query_row("SELECT count(*) FROM generated_audio_receipts", [], |r| {
                    r.get(0)
                })?;
            assert_eq!(count, 1);
            Ok(())
        })
        .unwrap();
    drop(catalog);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn tampering_and_invalid_destination_do_not_publish_a_source() {
    let (root, candidate) = fixture();
    let catalog = echo_catalog::open_catalog(&root.join("catalog.sqlite")).unwrap();
    assert!(accept_narration(&catalog, &candidate, "missing-project", false).is_err());
    assert!(
        catalog
            .with_transaction(echo_catalog::list_assets)
            .unwrap()
            .is_empty()
    );
    assert!(accept_narration(&catalog, &candidate, "", false).is_err());
    assert!(
        !root
            .join("media/generated")
            .join(&candidate.receipt.output_hash)
            .join("narration.wav")
            .exists()
    );
    fs::write(&candidate.path, b"changed after audition").unwrap();
    assert!(accept_narration(&catalog, &candidate, "", true).is_err());
    assert!(
        catalog
            .with_transaction(echo_catalog::list_assets)
            .unwrap()
            .is_empty()
    );
    drop(catalog);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn bounded_request_rejects_empty_and_oversized_text_before_runtime() {
    for text in ["".to_owned(), "  ".into(), "a".repeat(501), "a\0b".into()] {
        assert!(crate::infer_runtime::speech::validate_text(&text).is_err());
    }
    assert!(crate::infer_runtime::speech::validate_text(&"旁".repeat(500)).is_ok());
}

#[test]
#[ignore = "requires a live authorized local speech deployment and explicit output directory"]
fn live_narration_candidate_and_acceptance() {
    let root =
        PathBuf::from(std::env::var("ECHO_NARRATION_LIVE_ROOT").expect("explicit output root"));
    fs::create_dir_all(root.join("candidate")).unwrap();
    let candidate = generate_narration(
        "这是一段后来补充的旁白，记录那个安静的午后。",
        &root.join("candidate"),
        InferRuntimeConfig {
            base_url: std::env::var("ECHO_INFER_ENDPOINT").unwrap_or_default(),
            credential_path: crate::infer_runtime_credential_path().unwrap(),
        },
    )
    .unwrap();
    let catalog = echo_catalog::open_catalog(&root.join("library/catalog.sqlite")).unwrap();
    assert!(
        catalog
            .with_transaction(echo_catalog::list_assets)
            .unwrap()
            .is_empty()
    );
    let id = accept_narration(&catalog, &candidate, "", true).unwrap();
    fs::write(root.join("receipt.json"), candidate.details_json().unwrap()).unwrap();
    catalog
        .with_transaction(|tx| -> Result<(), CatalogError> {
            let summary = echo_catalog::asset_source_disclosure(tx, id.parse().unwrap())?;
            assert!(summary.has_generated_source());
            let membership = &echo_catalog::sound_memberships(tx)?[&id];
            assert!(membership.in_materials && !membership.in_memory);
            Ok(())
        })
        .unwrap();
}
