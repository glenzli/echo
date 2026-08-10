use std::{
    fs::Permissions,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use serde_json::json;
use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};

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
    write_registration(&root, "generation-a", "http://127.0.0.1:9111", 45);
    let mut resolver = EndpointResolver::with_runtime_root("http://127.0.0.1:9222", root.clone());

    let selection = resolver.resolve().expect("override is valid");

    assert_eq!(selection.origin, "http://127.0.0.1:9222");
    assert_eq!(selection.source, EndpointSource::ExplicitOverride);
    std::fs::remove_dir_all(root).expect("fixture removes");
}

#[test]
fn discovery_tracks_generation_and_lease() {
    let root = fixture_root("generation");
    write_registration(&root, "generation-a", "http://127.0.0.1:9111", 45);
    let mut resolver = EndpointResolver::with_runtime_root("", root.clone());

    let first = resolver.resolve().expect("first generation resolves");
    assert_eq!(first.source, EndpointSource::Discovery);
    assert_eq!(first.generation.as_deref(), Some("generation-a"));
    assert_eq!(first.origin, "http://127.0.0.1:9111");

    write_registration(&root, "generation-b", "http://127.0.0.1:9222", 45);
    let second = resolver.resolve().expect("new generation resolves");
    assert_eq!(second.generation.as_deref(), Some("generation-b"));
    assert_eq!(second.origin, "http://127.0.0.1:9222");

    write_registration(&root, "generation-b", "http://127.0.0.1:9222", -1);
    let expired = resolver.resolve().expect("fallback remains available");
    assert_eq!(expired.source, EndpointSource::CompatibilityFallback);
    assert_eq!(expired.origin, COMPATIBILITY_FALLBACK_ENDPOINT);
    std::fs::remove_dir_all(root).expect("fixture removes");
}

#[test]
fn connection_failure_refreshes_discovery_cache() {
    let root = fixture_root("connection-failure");
    write_registration(&root, "generation-a", "http://127.0.0.1:9111", 45);
    let mut resolver = EndpointResolver::with_runtime_root("", root.clone());
    resolver.resolve().expect("first generation resolves");
    write_registration(&root, "generation-b", "http://127.0.0.1:9222", 45);

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
    write_registration(&root, "generation-a", "http://127.0.0.1:9111", 45);
    let path = root.join("registrations").join("infer-runtime--local.json");
    std::fs::set_permissions(&path, Permissions::from_mode(0o644)).expect("permissions change");
    let mut resolver = EndpointResolver::with_runtime_root("", root.clone());

    let selection = resolver.resolve().expect("fallback remains available");

    assert_eq!(selection.source, EndpointSource::CompatibilityFallback);
    std::fs::remove_dir_all(root).expect("fixture removes");
}

#[test]
fn requires_exact_consumer_protocol_version() {
    let root = fixture_root("version");
    write_registration_value(
        &root,
        &registration_value(
            "generation-a",
            "http://127.0.0.1:9111",
            45,
            "0.1.0-candidate.1",
        ),
    );
    let mut resolver = EndpointResolver::with_runtime_root("", root.clone());

    let selection = resolver.resolve().expect("fallback remains available");

    assert_eq!(selection.source, EndpointSource::CompatibilityFallback);
    std::fs::remove_dir_all(root).expect("fixture removes");
}

fn fixture_root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "echo-discovery-{label}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let registrations = root.join("registrations");
    std::fs::create_dir_all(&registrations).expect("fixture directories create");
    std::fs::set_permissions(&root, Permissions::from_mode(0o700)).expect("root mode sets");
    std::fs::set_permissions(&registrations, Permissions::from_mode(0o700))
        .expect("registrations mode sets");
    root
}

fn write_registration(root: &Path, generation: &str, endpoint: &str, lease_seconds: i64) {
    write_registration_value(
        root,
        &registration_value(
            generation,
            endpoint,
            lease_seconds,
            CONSUMER_PROTOCOL_VERSION,
        ),
    );
}

fn registration_value(
    generation: &str,
    endpoint: &str,
    lease_seconds: i64,
    protocol_version: &str,
) -> serde_json::Value {
    let now = OffsetDateTime::now_utc();
    let renewed = now - Duration::seconds(1);
    let expires = now + Duration::seconds(lease_seconds);
    json!({
        "schema": DISCOVERY_SCHEMA,
        "schema_version": DISCOVERY_SCHEMA_VERSION,
        "service": {
            "kind": SERVICE_KIND,
            "instance_id": DEFAULT_INSTANCE_ID,
            "generation": generation
        },
        "lease": {
            "renewed_at": renewed.format(&Rfc3339).expect("time formats"),
            "expires_at": expires.format(&Rfc3339).expect("time formats")
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
