use std::path::Path;

use echo_domain::{
    AnalysisKind, AnalysisRecord, ContentHash, MetadataField, MetadataFields, ModelIdentity,
};

use super::*;
use crate::{
    AppendAnalysisRecord, AppendContextualAnalysis, AssetRegistrationInput, RegisterAsset,
    index_semantic_source_text, open_catalog, record_analysis, record_contextual_analysis,
    register_asset, search_semantic_text, search_transcripts, semantic_source,
};

#[test]
fn calibration_overrides_presentation_facets_and_literal_search_without_mutating_analysis() {
    let root =
        std::env::temp_dir().join(format!("echo-metadata-calibration-{}", std::process::id()));
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let asset = seed_model_evidence(&catalog);

    let desired = MetadataFields {
        sound_caption: "Morning kitchen rain".into(),
        summary: "A model summary".into(),
        event_type: "daily routine".into(),
        mood: String::new(),
        keywords: vec!["rain".into(), "kitchen".into()],
        transcript_text: "Water is boiling while rain taps the window.".into(),
        language: "en".into(),
    };
    let revision = catalog
        .with_transaction(|transaction| {
            calibrate_asset_metadata(transaction, asset.id, desired.clone(), &[], 4)
        })
        .expect("calibration writes");
    assert_eq!(
        revision.calibration.calibrated_fields(),
        [
            MetadataField::SoundCaption,
            MetadataField::EventType,
            MetadataField::Mood,
            MetadataField::Keywords,
            MetadataField::TranscriptText,
        ]
    );

    assert_effective_projections(&catalog, &desired);

    catalog
        .with_transaction(|transaction| {
            let model = model_metadata_fields(transaction, asset.id)?;
            let reset = calibrate_asset_metadata(transaction, asset.id, model, &[], 5)?;
            assert!(reset.calibration.is_empty());
            let projected = crate::list_audio_space(transaction)?;
            assert_eq!(
                projected[0].effective_metadata.sound_caption,
                "Rain at the window"
            );
            assert!(
                projected[0]
                    .metadata_calibration
                    .as_ref()
                    .is_some_and(|revision| revision.calibration.is_empty())
            );
            Ok::<_, crate::CatalogError>(())
        })
        .expect("reset restores model projection");

    let _ = std::fs::remove_dir_all(root);
}

fn seed_model_evidence(catalog: &crate::Catalog) -> echo_domain::AudioAsset {
    catalog
        .with_transaction(|transaction| -> Result<_, crate::CatalogError> {
            let asset = match register_asset(
                transaction,
                &AssetRegistrationInput {
                    content_hash: ContentHash::new([207; 32]),
                    path: Path::new("/sounds/rain.wav"),
                    size_bytes: 100,
                    codec: Some("pcm"),
                    duration_millis: Some(8_000),
                    recorded_at_millis: None,
                    imported_at_millis: 1,
                },
            )? {
                RegisterAsset::Created(asset) | RegisterAsset::Existed(asset) => asset,
            };
            record_analysis(
                transaction,
                &AppendAnalysisRecord {
                    asset_id: asset.id,
                    record: AnalysisRecord::new(
                        AnalysisKind::Transcript,
                        serde_json::json!({"text":"Light rain by the window.", "language":"en"}),
                        ModelIdentity::new("asr".into(), "1".into()),
                        None,
                        2,
                    ),
                },
            )?;
            crate::index_transcript(
                transaction,
                &asset.id.to_string(),
                "Light rain by the window.",
            )?;
            record_contextual_analysis(
                transaction,
                &AppendContextualAnalysis {
                    analysis: AppendAnalysisRecord {
                        asset_id: asset.id,
                        record: AnalysisRecord::new(
                            AnalysisKind::Contextual,
                            serde_json::json!({
                                "sound_caption":"Rain at the window",
                                "summary":"A model summary",
                                "keywords":["rain","window"],
                                "mood":"quiet",
                                "event_type":"weather"
                            }),
                            ModelIdentity::new("llm".into(), "1".into()),
                            None,
                            3,
                        ),
                    },
                },
            )?;
            let source = semantic_source(transaction, asset.id)?.expect("semantic source");
            index_semantic_source_text(transaction, &source)?;
            Ok(asset)
        })
        .expect("model evidence writes")
}

fn assert_effective_projections(catalog: &crate::Catalog, desired: &MetadataFields) {
    catalog
        .with_transaction(|transaction| {
            let projected = crate::list_audio_space(transaction)?;
            assert_eq!(
                projected[0].model_metadata.sound_caption,
                "Rain at the window"
            );
            assert_eq!(&projected[0].effective_metadata, desired);
            assert_eq!(
                crate::list_contextual_keyword_facets(transaction)?
                    .into_iter()
                    .map(|facet| facet.label)
                    .collect::<Vec<_>>(),
                ["kitchen", "rain"]
            );
            assert_eq!(search_transcripts(transaction, "boiling", 5)?.len(), 1);
            assert_eq!(search_transcripts(transaction, "Light rain", 5)?.len(), 0);
            assert_eq!(search_semantic_text(transaction, "kitchen", 5)?.len(), 1);
            Ok::<_, crate::CatalogError>(())
        })
        .expect("effective projections read");
}
