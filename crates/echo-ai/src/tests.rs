//! Model registry facade contracts: catalog integrity and HF-cache
//! resolution behavior.

use crate::{
    Capability, InferenceBackend, MODEL_CATALOG, ModelStatus, repo_cache_dir, resolve_model,
};
use std::path::PathBuf;

fn fixture_root(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "echo-ai-models-{}-{name}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("t")
    ))
}

#[test]
fn catalog_identities_are_unique_and_consistent() {
    let _root = fixture_root("catalog");
    let mut ids = std::collections::HashSet::new();
    for spec in MODEL_CATALOG {
        assert!(ids.insert(spec.id), "duplicate model id {}", spec.id);
        assert!(
            spec.repo.contains('/'),
            "repo must be org/name: {}",
            spec.repo
        );
        assert!(!spec.required_files.is_empty());
        assert!(matches!(
            spec.backend,
            InferenceBackend::Mlx | InferenceBackend::WhisperCpp
        ));
        assert!(
            matches!(
                spec.capability,
                Capability::Transcribe | Capability::Align | Capability::UnderstandAudio
            ),
            "M1 catalog must only declare M1 capabilities"
        );
    }
}

#[test]
fn missing_model_reports_the_download_command() {
    let root = fixture_root("missing");
    let spec = &MODEL_CATALOG[0];
    let status = resolve_model(&root, spec).expect("resolution reads the root");
    assert_eq!(
        status,
        ModelStatus::Missing {
            download_command: format!("hf download {}", spec.repo)
        }
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn present_model_resolves_to_its_snapshot() {
    let root = fixture_root("present");
    let spec = &MODEL_CATALOG[0];
    let repo_dir = repo_cache_dir(&root, spec);
    let snapshot = repo_dir.join("snapshots").join("abc123");
    std::fs::create_dir_all(&snapshot).expect("fixture dirs");
    for file in spec.required_files {
        std::fs::write(snapshot.join(file), b"fixture").expect("fixture file");
    }

    let status = resolve_model(&root, spec).expect("resolution reads the root");
    assert_eq!(
        status,
        ModelStatus::Present {
            snapshot: snapshot.clone()
        }
    );

    // A partial download (missing a required file) still reports missing.
    std::fs::remove_file(snapshot.join(spec.required_files[0])).expect("remove fixture");
    let status = resolve_model(&root, spec).expect("resolution reads the root");
    assert!(matches!(status, ModelStatus::Missing { .. }));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn revision_preference_wins_over_other_snapshots() {
    let root = fixture_root("revision");
    let mut spec = MODEL_CATALOG[0].clone();
    spec.revision = Some("pinned-rev");
    let repo_dir = repo_cache_dir(&root, &spec);
    for revision in ["other-rev", "pinned-rev"] {
        let snapshot = repo_dir.join("snapshots").join(revision);
        std::fs::create_dir_all(&snapshot).expect("fixture dirs");
        for file in spec.required_files {
            std::fs::write(snapshot.join(file), b"fixture").expect("fixture file");
        }
    }
    let status = resolve_model(&root, &spec).expect("resolution reads the root");
    assert_eq!(
        status,
        ModelStatus::Present {
            snapshot: repo_dir.join("snapshots").join("pinned-rev")
        }
    );
    let _ = std::fs::remove_dir_all(&root);
}
