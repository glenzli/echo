use std::path::Path;

use echo_domain::{
    AdjustmentGraph, AssemblyClip, AssemblyClipId, AssemblyMaster, AssemblyTrack, AssemblyTrackId,
    AssetId, ContentHash, FadeCurve, SoundAssembly, SoundAssemblyId,
};

use super::*;
use crate::{
    AssetRegistrationInput, RegisterAsset, open_catalog, record_adjustment_graph, register_asset,
};

#[test]
fn exact_asset_revision_and_source_bounds_are_durable() {
    let root = fixture_root("exact-source");
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let (asset_id, adjustment_id) = catalog
        .with_transaction(|transaction| {
            let asset_id = register(transaction, 0x31, "/sounds/rain.wav", 10_000);
            let adjustment = record_adjustment_graph(
                transaction,
                asset_id,
                AdjustmentGraph::identity(10_000).expect("identity adjustment"),
                4,
            )?;
            Ok::<_, CatalogError>((asset_id, adjustment.revision_id))
        })
        .expect("fixture writes");
    let assembly_id = SoundAssemblyId::new();
    let assembly = document(assembly_id, asset_id, adjustment_id, 7_500, "Rain bed");
    let first = catalog
        .with_transaction(|transaction| record_sound_assembly(transaction, &assembly, 10))
        .expect("assembly records");
    assert_eq!(first.revision_number, 1);
    let repeated = catalog
        .with_transaction(|transaction| record_sound_assembly(transaction, &assembly, 20))
        .expect("equivalent document is idempotent");
    assert_eq!(repeated.revision_id, first.revision_id);

    let loaded = catalog
        .with_transaction(|transaction| latest_sound_assembly(transaction, assembly.id()))
        .expect("assembly reads")
        .expect("assembly exists");
    assert_eq!(loaded.assembly, assembly);
    assert_eq!(list(&catalog)[0].duration_millis, 7_500);

    let invalid = document(assembly_id, asset_id, adjustment_id, 10_001, "Too long");
    let error = catalog
        .with_transaction(|transaction| record_sound_assembly(transaction, &invalid, 30))
        .expect_err("range beyond exact revision is rejected");
    assert!(error.message.contains("source range exceeds"));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn revisions_archive_and_generated_export_provenance_preserve_history() {
    let root = fixture_root("history");
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let asset_id = catalog
        .with_transaction(|transaction| {
            Ok::<_, CatalogError>(register(transaction, 0x42, "/sounds/door.wav", 4_000))
        })
        .expect("asset registers");
    let assembly_id = SoundAssemblyId::new();
    let first_document = document(assembly_id, asset_id, 0, 2_000, "Door sequence");
    let first = catalog
        .with_transaction(|transaction| record_sound_assembly(transaction, &first_document, 10))
        .expect("first revision records");
    let second_document = document(assembly_id, asset_id, 0, 3_000, "Door sequence");
    let second = catalog
        .with_transaction(|transaction| record_sound_assembly(transaction, &second_document, 20))
        .expect("second revision records");
    assert_eq!(second.revision_number, 2);
    assert_eq!(
        catalog
            .with_transaction(|transaction| {
                sound_assembly_at_revision(transaction, first_document.id(), first.revision_id)
            })
            .expect("historical revision reads")
            .expect("revision exists")
            .assembly,
        first_document
    );

    let export = catalog
        .with_transaction(|transaction| {
            record_sound_assembly_export(
                transaction,
                &RecordSoundAssemblyExport {
                    assembly_id: second_document.id(),
                    assembly_revision_id: second.revision_id,
                    output_path: Path::new("/exports/door.wav"),
                    format: SoundAssemblyExportFormat::WavPcm24,
                    sample_rate: 48_000,
                    channel_count: 2,
                    frame_count: 144_000,
                    content_hash: ContentHash::new([0x71; 32]),
                    size_bytes: 864_044,
                    integrated_lufs: -18.2,
                    true_peak_dbtp: -1.1,
                    created_at_millis: 30,
                },
            )
        })
        .expect("export records");
    let provenance: serde_json::Value =
        serde_json::from_str(&export.provenance_json).expect("provenance decodes");
    assert_eq!(provenance["assemblyRevisionId"], second.revision_id);
    assert_eq!(provenance["sources"][0]["assetId"], asset_id.to_string());
    assert_eq!(
        provenance["sources"][0]["adjustmentRevisionId"],
        serde_json::Value::Null
    );

    catalog
        .with_transaction(|transaction| {
            archive_sound_assembly(transaction, second_document.id(), 40)
        })
        .expect("assembly archives");
    assert!(list(&catalog).is_empty());
    assert!(
        catalog
            .with_transaction(|transaction| {
                latest_sound_assembly(transaction, second_document.id())
            })
            .expect("archived history reads")
            .is_some()
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn adjustment_revision_cannot_be_borrowed_from_another_asset() {
    let root = fixture_root("foreign-revision");
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let (first, second, second_revision) = catalog
        .with_transaction(|transaction| {
            let first = register(transaction, 0x51, "/sounds/first.wav", 2_000);
            let second = register(transaction, 0x52, "/sounds/second.wav", 2_000);
            let revision = record_adjustment_graph(
                transaction,
                second,
                AdjustmentGraph::identity(2_000).expect("identity adjustment"),
                2,
            )?;
            Ok::<_, CatalogError>((first, second, revision.revision_id))
        })
        .expect("fixtures write");
    assert_ne!(first, second);
    let error = catalog
        .with_transaction(|transaction| {
            record_sound_assembly(
                transaction,
                &document(
                    SoundAssemblyId::new(),
                    first,
                    second_revision,
                    1_000,
                    "Invalid",
                ),
                5,
            )
        })
        .expect_err("foreign revision is rejected");
    assert!(error.message.contains("does not belong"));
    let _ = std::fs::remove_dir_all(root);
}

fn document(
    assembly_id: SoundAssemblyId,
    asset_id: AssetId,
    adjustment_revision_id: i64,
    duration_millis: u64,
    name: &str,
) -> SoundAssembly {
    let clip = AssemblyClip::new(
        AssemblyClipId::new(),
        asset_id,
        adjustment_revision_id,
        0,
        duration_millis,
        0,
        0,
        0,
        0,
        0,
        FadeCurve::Linear,
        FadeCurve::Linear,
        false,
    )
    .expect("clip validates");
    let track = AssemblyTrack::new(
        AssemblyTrackId::new(),
        "Track 1".to_owned(),
        0,
        0,
        false,
        false,
        vec![clip],
    )
    .expect("track validates");
    SoundAssembly::new(
        assembly_id,
        name.to_owned(),
        AssemblyMaster::standard(),
        vec![track],
    )
    .expect("assembly validates")
}

fn register(
    transaction: &rusqlite::Transaction<'_>,
    byte: u8,
    path: &str,
    duration_millis: u64,
) -> AssetId {
    match register_asset(
        transaction,
        &AssetRegistrationInput {
            content_hash: ContentHash::new([byte; 32]),
            path: Path::new(path),
            size_bytes: 1_000,
            codec: Some("wav"),
            duration_millis: Some(duration_millis),
            recorded_at_millis: None,
            imported_at_millis: 1,
        },
    )
    .expect("asset registers")
    {
        RegisterAsset::Created(asset) | RegisterAsset::Existed(asset) => asset.id,
    }
}

fn list(catalog: &crate::Catalog) -> Vec<SoundAssemblySummary> {
    catalog
        .with_transaction(list_sound_assemblies)
        .expect("assemblies list")
}

fn fixture_root(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "echo-sound-assembly-{label}-{}-{}",
        std::process::id(),
        uuid::Uuid::now_v7(),
    ))
}
