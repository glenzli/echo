use super::*;

#[test]
fn prepared_prompt_is_exact_and_bound_to_the_original() {
    let prepared = PreparedSoundPrompt {
        original_prompt: "Rain, no music.".into(),
        effective_prompt: "Rain, no music.".into(),
        rules_revision: infer_runtime_client::SOUND_PROMPT_RULES_REVISION.into(),
        text_job: None,
        preparation_elapsed_ms: 0,
    };
    let json = serde_json::to_string(&prepared).unwrap();
    assert_eq!(
        decode_preparation(&json, "Rain, no music.")
            .unwrap()
            .effective_prompt,
        prepared.original_prompt
    );
    let mut legacy = prepared.clone();
    legacy.rules_revision = "infer.sound-prompt-preparation@20260926.1".into();
    assert!(legacy.validate_for(&legacy.original_prompt, "echo").is_ok());
    assert!(
        decode_preparation(
            &serde_json::to_string(&legacy).unwrap(),
            &legacy.original_prompt
        )
        .is_err()
    );
    assert!(decode_preparation(&json, "A door closes.").is_err());
    let mut value = serde_json::to_value(&prepared).unwrap();
    value["effective_prompt"] = "Rain and music.".into();
    assert!(decode_preparation(&value.to_string(), "Rain, no music.").is_err());
    value["original_prompt"] = "雨声，不要音乐".into();
    assert!(decode_preparation(&value.to_string(), "雨声，不要音乐").is_err());
}

#[test]
fn sound_choices_never_add_a_deployment_override() {
    for model in [
        SoundMaterialModel::SmallSfx,
        SoundMaterialModel::SmallMusic,
        SoundMaterialModel::OpenSmall,
    ] {
        let spec = SoundMaterialSpec {
            model,
            prompt: "A bell".into(),
            duration_seconds: 3,
            seed: 42,
            ambience: false,
        };
        let r = request(&spec).unwrap();
        assert!(r.model_choice.is_some());
        assert!(!r.metadata.contains_key("infer.deployment_ids"));
        assert!(!r.metadata.contains_key("infer.named_route"));
        assert_eq!(r.metadata["infer.placement"], "local_only");
        assert_eq!(r.metadata["infer.fallback"], "none");
    }
}
