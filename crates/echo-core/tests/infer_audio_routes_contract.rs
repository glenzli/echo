//! Opt-in public consumer verification after Runtime routing/SDK changes.
use echo_core::{
    AlignmentIntent, AudioEventDetectionIntent, AudioTextQueryEmbeddingIntent, ContextualIntent,
    InferRuntimeClient, InferRuntimeConfig, RuntimeProvenance, TextEmbeddingIntent,
    TranscriptionIntent, generate_narration, infer_runtime_credential_path,
};
use serde_json::{Value, json};
use std::{fs, path::Path};

fn record(
    root: &Path,
    rows: &mut Vec<Value>,
    intent: &str,
    result: Result<RuntimeProvenance, String>,
) {
    let row = match result {
        Ok(runtime) => json!({"intent":intent,"ok":true,"runtime":runtime}),
        Err(error) => json!({"intent":intent,"ok":false,"error":error}),
    };
    eprintln!("{intent}: {}", row["ok"]);
    rows.push(row);
    fs::write(
        root.join("routes.json"),
        serde_json::to_vec_pretty(rows).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "requires managed local Infer and an explicit new ECHO_ROUTE_LIVE_ROOT"]
fn existing_audio_routes_remain_usable() {
    let root = std::path::PathBuf::from(std::env::var("ECHO_ROUTE_LIVE_ROOT").unwrap());
    fs::create_dir_all(root.join("narration")).unwrap();
    let config = InferRuntimeConfig {
        base_url: String::new(),
        credential_path: infer_runtime_credential_path().unwrap(),
    };
    let client = InferRuntimeClient::new(config.clone());
    let mut rows = Vec::new();
    let text = "窗外下着小雨，我们坐在屋里，听着安静的雨声。";
    let narration = generate_narration(text, &root.join("narration"), config);
    record(
        &root,
        &mut rows,
        "speech.synthesize",
        narration
            .map(|c| {
                let receipt: Value = serde_json::from_str(&c.details_json().unwrap()).unwrap();
                fs::write(
                    root.join("narration-receipt.json"),
                    c.details_json().unwrap(),
                )
                .unwrap();
                serde_json::from_value(receipt["runtime"].clone()).unwrap()
            })
            .map_err(|e| e.to_string()),
    );
    let source = root.join("narration/narration.wav");
    record(
        &root,
        &mut rows,
        "audio.transcribe",
        client
            .transcribe(&source, &TranscriptionIntent::default())
            .map(|p| p.runtime.unwrap())
            .map_err(|e| e.to_string()),
    );
    record(
        &root,
        &mut rows,
        "audio.align",
        client
            .align(&source, text, &AlignmentIntent::default())
            .map(|p| p.runtime)
            .map_err(|e| e.to_string()),
    );
    record(
        &root,
        &mut rows,
        "audio.detect_events",
        client
            .detect_audio_events(&source, &AudioEventDetectionIntent::default())
            .map(|p| p.runtime)
            .map_err(|e| e.to_string()),
    );
    record(
        &root,
        &mut rows,
        "semantic.embed_text",
        client
            .embed_text(text, &TextEmbeddingIntent::new("echo-route-fixture-v1"))
            .map(|p| p.runtime)
            .map_err(|e| e.to_string()),
    );
    record(
        &root,
        &mut rows,
        "audio.embed",
        client
            .embed_audio(&source, "echo-route-fixture-v1")
            .map(|p| p.runtime)
            .map_err(|e| e.to_string()),
    );
    record(
        &root,
        &mut rows,
        "audio.embed_text_query",
        client
            .embed_audio_text_query(
                "轻柔的雨声",
                &AudioTextQueryEmbeddingIntent::new("echo-route-query-v1", "zh"),
            )
            .map(|p| p.runtime)
            .map_err(|e| e.to_string()),
    );
    record(
        &root,
        &mut rows,
        "text.summarize",
        client
            .contextualize(text, &ContextualIntent::default())
            .map(|p| p.runtime)
            .map_err(|e| e.to_string()),
    );
    assert!(
        rows.iter().all(|r| r["ok"] == true),
        "inspect routes.json for failed contracts"
    );
}
