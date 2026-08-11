use std::{
    fs::Permissions,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use serde_json::json;

use super::*;

#[test]
fn accepts_only_canonical_numeric_loopback_origins() {
    for origin in [
        "http://127.0.0.1:8787",
        "http://127.23.4.5:1",
        "http://[::1]:8787",
    ] {
        assert_eq!(canonical_loopback_origin(origin).as_deref(), Ok(origin));
    }
    for origin in [
        "https://127.0.0.1:8787",
        "http://localhost:8787",
        "http://0.0.0.0:8787",
        "http://192.168.1.2:8787",
        "http://127.0.0.1",
        "http://127.0.0.1:0",
        "http://127.0.0.1:8787/",
        "http://127.0.0.1:8787/path",
        "http://user@127.0.0.1:8787",
        "http://127.0.0.1:8787?query",
        " http://127.0.0.1:8787",
    ] {
        assert!(
            canonical_loopback_origin(origin).is_err(),
            "accepted {origin}"
        );
    }
}

#[test]
fn explicit_override_wins_over_discovery() {
    let root = fixture_root("override");
    write_registration(&root, "generation-a", "http://127.0.0.1:9111");
    let mut resolver = EndpointResolver::with_runtime_root("http://127.0.0.1:9222", root.clone());

    let selection = resolver.resolve().expect("override is valid");

    assert_eq!(selection.origin, "http://127.0.0.1:9222");
    assert_eq!(selection.source, EndpointSource::ExplicitOverride);
    std::fs::remove_dir_all(root).expect("fixture removes");
}

#[test]
fn invalid_override_is_distinct_from_missing_discovery() {
    let root = fixture_root("invalid-override");
    let mut resolver = EndpointResolver::with_runtime_root("http://localhost:8787", root.clone());

    assert_eq!(resolver.resolve(), Err(EndpointError::InvalidEndpoint));

    let mut resolver = EndpointResolver::with_runtime_root("", root.clone());
    assert_eq!(resolver.resolve(), Err(EndpointError::DiscoveryUnavailable));
    std::fs::remove_dir_all(root).expect("fixture removes");
}

#[test]
fn discovery_tracks_generation_and_offer_without_liveness_timestamps() {
    let root = fixture_root("generation");
    write_registration(&root, "generation-a", "http://127.0.0.1:9111");
    let mut resolver = EndpointResolver::with_runtime_root("", root.clone());

    let first = resolver.resolve().expect("first generation resolves");
    assert_eq!(first.source, EndpointSource::Discovery);
    assert_eq!(
        first.protocol_version.as_deref(),
        Some(CONSUMER_PROTOCOL_VERSION)
    );
    assert_eq!(first.generation.as_deref(), Some("generation-a"));
    assert_eq!(first.origin, "http://127.0.0.1:9111");

    write_registration(&root, "generation-b", "http://127.0.0.1:9222");
    let second = resolver.resolve().expect("new generation resolves");
    assert_eq!(second.generation.as_deref(), Some("generation-b"));
    assert_eq!(second.origin, "http://127.0.0.1:9222");
    std::fs::remove_dir_all(root).expect("fixture removes");
}

#[test]
fn connection_failure_refreshes_discovery_cache() {
    let root = fixture_root("connection-failure");
    write_registration(&root, "generation-a", "http://127.0.0.1:9111");
    let mut resolver = EndpointResolver::with_runtime_root("", root.clone());
    resolver.resolve().expect("first generation resolves");
    write_registration(&root, "generation-b", "http://127.0.0.1:9222");

    resolver.connection_failed();

    let cached = resolver
        .cached_discovery
        .as_ref()
        .expect("connection failure immediately re-discovers");
    assert_eq!(cached.generation.as_deref(), Some("generation-b"));
    assert_eq!(cached.origin, "http://127.0.0.1:9222");
    std::fs::remove_dir_all(root).expect("fixture removes");
}

#[test]
fn rejects_registration_that_is_not_owner_only() {
    let root = fixture_root("permissions");
    write_registration(&root, "generation-a", "http://127.0.0.1:9111");
    let path = root.join("registrations").join("infer-runtime--local.json");
    std::fs::set_permissions(&path, Permissions::from_mode(0o644)).expect("permissions change");
    let mut resolver = EndpointResolver::with_runtime_root("", root.clone());

    assert!(resolver.resolve().is_err());
    std::fs::remove_dir_all(root).expect("fixture removes");
}

#[test]
fn rejects_a_runtime_root_without_the_shared_socket_directory() {
    let root = fixture_root("missing-sockets");
    write_registration(&root, "generation-a", "http://127.0.0.1:9111");
    std::fs::remove_dir(root.join("sockets")).expect("socket directory removes");
    let mut resolver = EndpointResolver::with_runtime_root("", root.clone());

    assert!(resolver.resolve().is_err());
    std::fs::remove_dir_all(root).expect("fixture removes");
}

#[test]
fn requires_exact_consumer_protocol_version() {
    let root = fixture_root("version");
    write_registration_value(
        &root,
        &registration_value("generation-a", "http://127.0.0.1:9111", "0.1.0-candidate.1"),
    );
    let mut resolver = EndpointResolver::with_runtime_root("", root.clone());

    assert!(resolver.resolve().is_err());
    std::fs::remove_dir_all(root).expect("fixture removes");
}

#[test]
fn selects_candidate4_from_an_exact_version_set() {
    let root = fixture_root("version-intersection");
    let mut value = registration_value(
        "generation-a",
        "http://127.0.0.1:9111",
        CONSUMER_PROTOCOL_VERSION,
    );
    value["offers"][0]["protocol_versions"] =
        serde_json::json!([CONSUMER_PROTOCOL_VERSION, "0.1.0-obsolete"]);
    write_registration_value(&root, &value);
    let mut resolver = EndpointResolver::with_runtime_root("", root.clone());

    let selection = resolver
        .resolve()
        .expect("candidate4 intersection resolves");
    assert_eq!(
        selection.protocol_version.as_deref(),
        Some(CONSUMER_PROTOCOL_VERSION)
    );
    std::fs::remove_dir_all(root).expect("fixture removes");
}

#[test]
fn rejects_duplicate_consumer_protocol_versions() {
    let root = fixture_root("duplicate-version");
    let mut value = registration_value(
        "generation-a",
        "http://127.0.0.1:9111",
        CONSUMER_PROTOCOL_VERSION,
    );
    value["offers"][0]["protocol_versions"] =
        serde_json::json!([CONSUMER_PROTOCOL_VERSION, CONSUMER_PROTOCOL_VERSION]);
    write_registration_value(&root, &value);
    let mut resolver = EndpointResolver::with_runtime_root("", root.clone());

    assert!(resolver.resolve().is_err());
    std::fs::remove_dir_all(root).expect("fixture removes");
}

#[test]
fn rejects_unknown_registration_fields() {
    let root = fixture_root("unknown-field");
    let mut value = registration_value(
        "generation-a",
        "http://127.0.0.1:9111",
        CONSUMER_PROTOCOL_VERSION,
    );
    value["obsolete"] = json!({});
    write_registration_value(&root, &value);
    let mut resolver = EndpointResolver::with_runtime_root("", root.clone());

    assert!(resolver.resolve().is_err());
    std::fs::remove_dir_all(root).expect("fixture removes");
}

fn fixture_root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "echo-discovery-{label}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let registrations = root.join("registrations");
    let sockets = root.join("sockets");
    std::fs::create_dir_all(&registrations).expect("fixture directories create");
    std::fs::create_dir_all(&sockets).expect("fixture socket directory creates");
    std::fs::set_permissions(&root, Permissions::from_mode(0o700)).expect("root mode sets");
    std::fs::set_permissions(&registrations, Permissions::from_mode(0o700))
        .expect("registrations mode sets");
    std::fs::set_permissions(&sockets, Permissions::from_mode(0o700)).expect("sockets mode sets");
    root
}

fn write_registration(root: &Path, generation: &str, endpoint: &str) {
    write_registration_value(
        root,
        &registration_value(generation, endpoint, CONSUMER_PROTOCOL_VERSION),
    );
}

fn registration_value(
    generation: &str,
    endpoint: &str,
    protocol_version: &str,
) -> serde_json::Value {
    json!({
        "schema": DISCOVERY_SCHEMA,
        "schema_version": DISCOVERY_SCHEMA_VERSION,
        "service": {
            "kind": SERVICE_KIND,
            "instance_id": DEFAULT_INSTANCE_ID,
            "generation": generation
        },
        "offers": [{
            "protocol": CONSUMER_PROTOCOL,
            "protocol_versions": [protocol_version],
            "binding": CONSUMER_BINDING,
            "endpoint": endpoint
        }]
    })
}

fn write_registration_value(root: &Path, value: &serde_json::Value) {
    let path = root.join("registrations").join("infer-runtime--local.json");
    std::fs::write(&path, serde_json::to_vec(&value).expect("JSON encodes"))
        .expect("manifest writes");
    std::fs::set_permissions(path, Permissions::from_mode(0o600)).expect("manifest mode sets");
}
