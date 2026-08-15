use std::{fs, path::Path};

use echo_catalog::{
    AssetRegistrationInput, RegisterAsset, record_adjustment_graph, register_asset,
};
use echo_domain::{AdjustmentGraph, ContentHash};

#[test]
fn rendered_working_copy_moves_private_render_to_cache_and_reports_upstream_staleness() {
    let root = std::env::temp_dir().join(format!(
        "echo-desktop-rendered-working-copy-{}",
        uuid::Uuid::now_v7()
    ));
    let source = root.join("source.wav");
    let rendered = root.join("rendered.wav");
    let erased = root.join("erased.wav");
    fs::create_dir_all(&root).expect("fixture root creates");
    fs::write(&source, b"immutable source").expect("source writes");
    fs::write(&rendered, b"frozen processed render").expect("render writes");
    fs::write(&erased, b"processed render after spectral erase").expect("erase writes");
    let session = super::super::open_session(
        root.join("catalog.sqlite")
            .to_str()
            .expect("utf8 catalog path"),
        root.join("cache").to_str().expect("utf8 cache path"),
    )
    .expect("session opens");
    let (asset_id, revision) = session
        .catalog()
        .with_transaction(|transaction| -> Result<_, echo_catalog::CatalogError> {
            let asset_id = match register_asset(
                transaction,
                &AssetRegistrationInput {
                    content_hash: ContentHash::new([3; 32]),
                    path: Path::new(&source),
                    size_bytes: 16,
                    codec: Some("pcm"),
                    duration_millis: Some(1_000),
                    recorded_at_millis: None,
                    imported_at_millis: 1,
                },
            )? {
                RegisterAsset::Created(asset) | RegisterAsset::Existed(asset) => asset.id,
            };
            let revision = record_adjustment_graph(
                transaction,
                asset_id,
                AdjustmentGraph::identity(1_000).expect("identity graph validates"),
                2,
            )?;
            Ok((asset_id, revision.revision_id))
        })
        .expect("asset and adjustment record");
    let created = session
        .create_rendered_spectral_working_copy(
            &asset_id.to_string(),
            revision,
            rendered.to_str().expect("utf8 rendered path"),
        )
        .expect("working copy admits");
    assert!(created.cache_path.is_file());
    assert!(created.record.enabled);
    assert_eq!(
        fs::read(&created.cache_path).expect("cached bytes read"),
        b"frozen processed render"
    );

    let committed = session
        .commit_rendered_spectral_erase(
            &asset_id.to_string(),
            created.record.id,
            erased.to_str().expect("utf8 erase path"),
            100,
            220,
            240,
            1_800,
            9_600,
            12,
            48,
        )
        .expect("erase advances the shared working cache");
    assert_eq!(committed.record.operation_count, 1);
    assert_eq!(
        fs::read(&committed.cache_path).expect("committed bytes read"),
        b"processed render after spectral erase"
    );

    session
        .catalog()
        .with_transaction(|transaction| {
            record_adjustment_graph(
                transaction,
                asset_id,
                AdjustmentGraph::new(
                    1_000,
                    20,
                    980,
                    0,
                    0,
                    echo_domain::AdjustmentEffects::default(),
                )
                .expect("changed graph validates"),
                3,
            )
        })
        .expect("upstream changes");
    let listed = session
        .rendered_spectral_working_copies(&asset_id.to_string())
        .expect("working copies list");
    assert_eq!(listed.len(), 1);
    assert_eq!(
        listed[0].record.availability,
        echo_catalog::RenderedSpectralWorkingCopyAvailability::UpstreamChanged
    );
    let _ = fs::remove_dir_all(root);
}
