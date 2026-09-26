use super::*;
use echo_catalog::{CatalogError, SourceDisclosureOrigin};

fn fixture() -> (PathBuf, GeneratedAudioCandidate) {
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
    let id = accept_generated_audio(&catalog, &candidate, "", true).unwrap();
    assert_eq!(
        accept_generated_audio(&catalog, &candidate, "", true).unwrap(),
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
    assert!(accept_generated_audio(&catalog, &candidate, "missing-project", false).is_err());
    assert!(
        catalog
            .with_transaction(echo_catalog::list_assets)
            .unwrap()
            .is_empty()
    );
    assert!(accept_generated_audio(&catalog, &candidate, "", false).is_err());
    assert!(
        !root
            .join("media/generated")
            .join(&candidate.receipt.output_hash)
            .join("narration.wav")
            .exists()
    );
    fs::write(&candidate.path, b"changed after audition").unwrap();
    assert!(accept_generated_audio(&catalog, &candidate, "", true).is_err());
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
    for text in [String::new(), "  ".into(), "a".repeat(501), "a\0b".into()] {
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
    let id = accept_generated_audio(&catalog, &candidate, "", true).unwrap();
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

fn sound_fixture(ambience: bool, model: SoundMaterialModel) -> (PathBuf, GeneratedAudioCandidate) {
    let (root, narration) = fixture();
    let bytes = fs::read(&narration.path).unwrap();
    let mut runtime = narration.receipt.runtime;
    runtime.job.id = "test-sound-generation".into();
    runtime.job.intent = "audio.generate_sound".into();
    runtime.job.physical_model = "test-sfx".into();
    let spec = SoundMaterialSpec {
        model,
        prompt: "Gentle rain\n outside a window.".into(),
        duration_seconds: 1,
        seed: u32::MAX,
        ambience,
    };
    let request =
        serde_json::to_value(crate::infer_runtime::sound_generation::request(&spec).unwrap())
            .unwrap();
    let candidate = stage_sound_material(&spec, &root, &bytes, runtime, request).unwrap();
    (root, candidate)
}

#[test]
fn sound_effect_is_private_durable_and_binds_prompt_seed_category_and_bytes() {
    for (ambience, model) in [
        (false, SoundMaterialModel::SmallSfx),
        (true, SoundMaterialModel::SmallSfx),
        (false, SoundMaterialModel::SmallMusic),
        (true, SoundMaterialModel::OpenSmall),
    ] {
        let (root, candidate) = sound_fixture(ambience, model);
        let catalog = echo_catalog::open_catalog(&root.join("project/catalog.sqlite")).unwrap();
        catalog
            .with_transaction(|tx| -> Result<(), CatalogError> {
                tx.execute(
                    "INSERT INTO catalog_meta VALUES('session_kind','independent-editor-v1')",
                    [],
                )?;
                Ok(())
            })
            .unwrap();
        let id = accept_generated_audio(&catalog, &candidate, "", true).unwrap();
        fs::remove_file(&candidate.path).unwrap();
        catalog
            .with_transaction(|tx| -> Result<(), CatalogError> {
                let assets = echo_catalog::list_assets(tx)?;
                assert_eq!(assets.len(), 1);
                let path = root.join("project").join(&assets[0].original.path);
                assert!(path.ends_with("sound.wav"));
                assert_eq!(
                    crate::hash_file(&path).unwrap().to_string(),
                    candidate.receipt.output_hash
                );
                let memberships = echo_catalog::sound_memberships(tx)?;
                assert!(!memberships[&id].in_memory && !memberships[&id].in_materials);
                let disclosure = echo_catalog::asset_source_disclosure(tx, assets[0].id)?;
                assert!(disclosure.has_generated_source());
                assert!(disclosure.portable_comment().contains("ai_generated"));
                let receipt = disclosure.sources[0].generation.as_ref().unwrap();
                assert_eq!(
                    receipt["generation_kind"],
                    if model == SoundMaterialModel::SmallMusic {
                        "music"
                    } else {
                        "sound_effect"
                    }
                );
                assert_eq!(receipt["request"]["model_choice"], model.choice());
                assert_eq!(receipt["input_text"], "Gentle rain outside a window.");
                assert_eq!(receipt["request"]["seed"], u32::MAX);
                assert_eq!(
                    receipt["material_category"],
                    if model == SoundMaterialModel::SmallMusic {
                        "music"
                    } else if ambience {
                        "ambience"
                    } else {
                        "effects"
                    }
                );
                assert!(
                    echo_catalog::record_source_disclosure(tx, assets[0].id, 0, &[], 1).is_err()
                );
                Ok(())
            })
            .unwrap();
        drop(catalog);
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn inconsistent_sound_receipts_cannot_publish_assets() {
    for field in [
        "model",
        "prompt",
        "duration_seconds",
        "seed",
        "kind",
        "category",
        "model_choice",
    ] {
        let (root, mut candidate) = sound_fixture(false, SoundMaterialModel::SmallSfx);
        match field {
            "kind" => candidate.receipt.generation_kind = Some("narration".into()),
            "category" => candidate.receipt.material_category = Some("voice".into()),
            "seed" => candidate.receipt.request[field] = serde_json::json!(-1),
            "duration_seconds" => candidate.receipt.request[field] = serde_json::json!(30),
            _ => candidate.receipt.request[field] = serde_json::json!("mismatch"),
        }
        let catalog = echo_catalog::open_catalog(&root.join("catalog.sqlite")).unwrap();
        assert!(
            accept_generated_audio(&catalog, &candidate, "", true).is_err(),
            "{field}"
        );
        assert!(
            catalog
                .with_transaction(echo_catalog::list_assets)
                .unwrap()
                .is_empty()
        );
        drop(catalog);
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn sound_material_bounds_and_actual_duration_are_enforced() {
    let (root, candidate) = sound_fixture(false, SoundMaterialModel::SmallSfx);
    let mut spec = SoundMaterialSpec {
        model: SoundMaterialModel::SmallSfx,
        prompt: "rain".into(),
        duration_seconds: 1,
        seed: 42,
        ambience: false,
    };
    for seconds in [0, 31, u32::MAX] {
        spec.duration_seconds = seconds;
        assert!(spec.validate().is_err());
    }
    spec.duration_seconds = 1;
    for prompt in [String::new(), "a".repeat(501), "rain\0".into()] {
        spec.prompt = prompt;
        assert!(spec.validate().is_err());
    }
    spec.prompt = "雨".repeat(500);
    assert!(spec.validate().is_ok());
    spec.duration_seconds = 2;
    let other = root.join("other");
    fs::create_dir(&other).unwrap();
    assert!(
        stage_sound_material(
            &spec,
            &other,
            &fs::read(&candidate.path).unwrap(),
            candidate.receipt.runtime,
            serde_json::json!({})
        )
        .is_err()
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn legacy_sound_receipts_and_model_specific_limits_remain_valid() {
    let (root, mut candidate) = sound_fixture(false, SoundMaterialModel::SmallSfx);
    candidate
        .receipt
        .request
        .as_object_mut()
        .unwrap()
        .remove("model_choice");
    let catalog = echo_catalog::open_catalog(&root.join("catalog.sqlite")).unwrap();
    assert!(accept_generated_audio(&catalog, &candidate, "", true).is_ok());
    let mut spec = SoundMaterialSpec {
        model: SoundMaterialModel::OpenSmall,
        prompt: "door".into(),
        duration_seconds: 11,
        seed: 42,
        ambience: false,
    };
    assert!(spec.validate().is_ok());
    spec.duration_seconds = 12;
    assert!(spec.validate().is_err());
    spec.model = SoundMaterialModel::SmallMusic;
    spec.duration_seconds = 30;
    assert!(spec.validate().is_ok());
    assert!(SoundMaterialModel::from_choice("unknown").is_err());
    drop(catalog);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn music_receipt_cannot_be_relabelled_as_effects() {
    let (root, mut candidate) = sound_fixture(false, SoundMaterialModel::SmallMusic);
    candidate.receipt.material_category = Some("effects".into());
    let catalog = echo_catalog::open_catalog(&root.join("catalog.sqlite")).unwrap();
    assert!(accept_generated_audio(&catalog, &candidate, "", true).is_err());
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
fn prepared_receipt_survives_admission_and_rejects_detached_effective_prompt() {
    let (root, mut candidate) = sound_fixture(false, SoundMaterialModel::SmallSfx);
    let catalog = echo_catalog::open_catalog(&root.join("project/catalog.sqlite")).unwrap();
    candidate.receipt.schema_version = 3;
    candidate.receipt.prompt_preparation = Some(infer_runtime_client::PreparedSoundPrompt {
        original_prompt: candidate.receipt.input_text.clone(),
        effective_prompt: candidate.receipt.input_text.clone(),
        rules_revision: infer_runtime_client::SOUND_PROMPT_RULES_REVISION.into(),
        text_job: None,
        preparation_elapsed_ms: 0,
    });
    candidate
        .receipt
        .prompt_preparation
        .as_mut()
        .unwrap()
        .rules_revision = "infer.sound-prompt-preparation@20260926.1".into();
    assert!(accept_generated_audio(&catalog, &candidate, "", true).is_err());
    candidate
        .receipt
        .prompt_preparation
        .as_mut()
        .unwrap()
        .rules_revision = infer_runtime_client::SOUND_PROMPT_RULES_REVISION.into();
    let original = candidate.receipt.request["prompt"].clone();
    candidate.receipt.request["prompt"] = "unrelated sound".into();
    assert!(accept_generated_audio(&catalog, &candidate, "", true).is_err());
    candidate.receipt.request["prompt"] = original;
    let id = accept_generated_audio(&catalog, &candidate, "", true).unwrap();
    catalog
        .with_transaction(|tx| -> Result<(), CatalogError> {
            let value: String = tx.query_row(
                "SELECT receipt_json FROM generated_audio_receipts WHERE asset_id=?1",
                [&id],
                |r| r.get(0),
            )?;
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&value).unwrap(),
                serde_json::to_value(&candidate.receipt).unwrap()
            );
            let asset = &echo_catalog::list_assets(tx)?[0];
            assert!(asset.original.path.ends_with("sound.wav"));
            assert!(echo_catalog::asset_source_disclosure(tx, asset.id)?.has_generated_source());
            Ok(())
        })
        .unwrap();
    drop(catalog);
    fs::remove_dir_all(root).unwrap();
}
