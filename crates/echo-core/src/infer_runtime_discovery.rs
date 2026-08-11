//! Infra Discovery consumer for Infer Runtime's loopback HTTP endpoint.
//!
//! This owner validates the shared registration boundary and selects only the
//! exact Consumer protocol Echo implements. It never handles App identity or
//! credentials; those remain on the Infer Runtime wire owner.

use std::{
    collections::BTreeSet,
    net::SocketAddr,
    path::{Path, PathBuf},
    process::Command,
};

use serde::Deserialize;

const DISCOVERY_SCHEMA: &str = "infra.discovery.registration";
const DISCOVERY_SCHEMA_VERSION: &str = "20260812.1";
const SERVICE_KIND: &str = "infer-runtime";
const DEFAULT_INSTANCE_ID: &str = "local";
const CONSUMER_PROTOCOL: &str = "infer-runtime.consumer";
const CONSUMER_PROTOCOL_VERSION: &str = "0.1.0-candidate.4";
const CONSUMER_BINDING: &str = "infer-runtime.http-loopback";
const MAX_REGISTRATION_BYTES: u64 = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EndpointSource {
    ExplicitOverride,
    Discovery,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedEndpoint {
    pub(crate) origin: String,
    pub(crate) source: EndpointSource,
    pub(crate) protocol_version: Option<String>,
    pub(crate) instance_id: Option<String>,
    pub(crate) generation: Option<String>,
}

#[derive(Debug)]
pub(crate) struct EndpointResolver {
    explicit_override: Option<String>,
    runtime_root_override: Option<PathBuf>,
    cached_discovery: Option<ResolvedEndpoint>,
}

impl EndpointResolver {
    pub(crate) fn new(explicit_override: &str) -> Self {
        let explicit_override =
            (!explicit_override.trim().is_empty()).then(|| explicit_override.to_owned());
        Self {
            explicit_override,
            runtime_root_override: None,
            cached_discovery: None,
        }
    }

    #[cfg(test)]
    fn with_runtime_root(explicit_override: &str, runtime_root: PathBuf) -> Self {
        let mut resolver = Self::new(explicit_override);
        resolver.runtime_root_override = Some(runtime_root);
        resolver
    }

    pub(crate) fn resolve(&mut self) -> Result<ResolvedEndpoint, EndpointError> {
        if let Some(origin) = &self.explicit_override {
            return Ok(ResolvedEndpoint {
                origin: canonical_loopback_origin(origin)?,
                source: EndpointSource::ExplicitOverride,
                protocol_version: None,
                instance_id: None,
                generation: None,
            });
        }

        if let Some(selection) = self.discover() {
            if let Some(cached) = &self.cached_discovery
                && cached.instance_id == selection.instance_id
                && cached.generation == selection.generation
                && cached.origin == selection.origin
                && cached.protocol_version == selection.protocol_version
            {
                return Ok(cached.clone());
            }
            self.cached_discovery = Some(selection.clone());
            return Ok(selection);
        }
        self.cached_discovery = None;
        Err(EndpointError::DiscoveryUnavailable)
    }

    pub(crate) fn connection_failed(&mut self) {
        self.cached_discovery = None;
        if self.explicit_override.is_none() {
            self.cached_discovery = self.discover();
        }
    }

    fn discover(&self) -> Option<ResolvedEndpoint> {
        let root = self
            .runtime_root_override
            .clone()
            .or_else(discovery_runtime_root)?;
        let uid = current_effective_uid()?;
        validate_owner_directory(&root, uid).ok()?;
        let registrations = root.join("registrations");
        validate_owner_directory(&registrations, uid).ok()?;
        #[cfg(unix)]
        validate_owner_directory(&root.join("sockets"), uid).ok()?;
        let manifest_path = registrations.join("infer-runtime--local.json");
        validate_owner_file(&manifest_path, uid).ok()?;
        let bytes = std::fs::read(&manifest_path).ok()?;
        if bytes.len() > usize::try_from(MAX_REGISTRATION_BYTES).ok()? {
            return None;
        }
        let registration: Registration = serde_json::from_slice(&bytes).ok()?;
        registration.select_consumer().ok()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EndpointError {
    DiscoveryUnavailable,
    InvalidEndpoint,
}

pub(crate) fn canonical_loopback_origin(origin: &str) -> Result<String, EndpointError> {
    if origin.is_empty() || origin.trim() != origin {
        return Err(EndpointError::InvalidEndpoint);
    }
    let authority = origin
        .strip_prefix("http://")
        .ok_or(EndpointError::InvalidEndpoint)?;
    if authority.is_empty()
        || authority.contains(['/', '?', '#', '@'])
        || authority.chars().any(char::is_whitespace)
    {
        return Err(EndpointError::InvalidEndpoint);
    }
    let address = authority
        .parse::<SocketAddr>()
        .map_err(|_| EndpointError::InvalidEndpoint)?;
    if !address.ip().is_loopback() || address.port() == 0 {
        return Err(EndpointError::InvalidEndpoint);
    }
    let canonical = format!("http://{address}");
    (origin == canonical)
        .then_some(canonical)
        .ok_or(EndpointError::InvalidEndpoint)
}

fn discovery_runtime_root() -> Option<PathBuf> {
    if let Some(root) = std::env::var_os("INFRA_PROTOCOL_RUNTIME_DIR") {
        let root = PathBuf::from(root);
        return root.is_absolute().then_some(root);
    }
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("/usr/bin/getconf")
            .arg("DARWIN_USER_TEMP_DIR")
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let base = String::from_utf8(output.stdout).ok()?;
        let base = PathBuf::from(base.trim());
        return base.is_absolute().then(|| base.join("infra-protocol"));
    }
    #[cfg(target_os = "linux")]
    {
        let base = PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR")?);
        return base.is_absolute().then(|| base.join("infra-protocol"));
    }
    #[allow(unreachable_code)]
    None
}

#[cfg(unix)]
fn current_effective_uid() -> Option<u32> {
    let output = Command::new("/usr/bin/id").arg("-u").output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()?.trim().parse().ok()
}

#[cfg(not(unix))]
fn current_effective_uid() -> Option<u32> {
    None
}

#[cfg(unix)]
fn validate_owner_directory(path: &Path, uid: u32) -> Result<(), EndpointError> {
    use std::os::unix::fs::MetadataExt;

    let metadata =
        std::fs::symlink_metadata(path).map_err(|_| EndpointError::DiscoveryUnavailable)?;
    if !metadata.file_type().is_dir()
        || metadata.file_type().is_symlink()
        || metadata.uid() != uid
        || metadata.mode() & 0o7777 != 0o700
    {
        return Err(EndpointError::DiscoveryUnavailable);
    }
    Ok(())
}

#[cfg(not(unix))]
fn validate_owner_directory(_: &Path, _: u32) -> Result<(), EndpointError> {
    Err(EndpointError::DiscoveryUnavailable)
}

#[cfg(unix)]
fn validate_owner_file(path: &Path, uid: u32) -> Result<(), EndpointError> {
    use std::os::unix::fs::MetadataExt;

    let metadata =
        std::fs::symlink_metadata(path).map_err(|_| EndpointError::DiscoveryUnavailable)?;
    if !metadata.file_type().is_file()
        || metadata.file_type().is_symlink()
        || metadata.uid() != uid
        || metadata.mode() & 0o7777 != 0o600
        || metadata.len() > MAX_REGISTRATION_BYTES
    {
        return Err(EndpointError::DiscoveryUnavailable);
    }
    Ok(())
}

#[cfg(not(unix))]
fn validate_owner_file(_: &Path, _: u32) -> Result<(), EndpointError> {
    Err(EndpointError::DiscoveryUnavailable)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Registration {
    schema: String,
    schema_version: String,
    service: Service,
    offers: Vec<Offer>,
}

impl Registration {
    fn select_consumer(&self) -> Result<ResolvedEndpoint, EndpointError> {
        if self.schema != DISCOVERY_SCHEMA
            || self.schema_version != DISCOVERY_SCHEMA_VERSION
            || self.service.kind != SERVICE_KIND
            || self.service.instance_id != DEFAULT_INSTANCE_ID
            || !valid_service_kind(&self.service.kind)
            || !valid_file_token(&self.service.instance_id)
            || !valid_file_token(&self.service.generation)
            || self.offers.is_empty()
            || self.offers.len() > 64
        {
            return Err(EndpointError::DiscoveryUnavailable);
        }

        let mut selected = None;
        for offer in &self.offers {
            offer.validate()?;
            if offer.protocol == CONSUMER_PROTOCOL
                && offer.binding == CONSUMER_BINDING
                && offer
                    .protocol_versions
                    .iter()
                    .any(|version| version == CONSUMER_PROTOCOL_VERSION)
            {
                if selected.is_some() {
                    return Err(EndpointError::DiscoveryUnavailable);
                }
                selected = Some(canonical_loopback_origin(&offer.endpoint)?);
            }
        }
        let origin = selected.ok_or(EndpointError::DiscoveryUnavailable)?;
        Ok(ResolvedEndpoint {
            origin,
            source: EndpointSource::Discovery,
            protocol_version: Some(CONSUMER_PROTOCOL_VERSION.to_owned()),
            instance_id: Some(self.service.instance_id.clone()),
            generation: Some(self.service.generation.clone()),
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Service {
    kind: String,
    instance_id: String,
    generation: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Offer {
    protocol: String,
    protocol_versions: Vec<String>,
    binding: String,
    endpoint: String,
}

impl Offer {
    fn validate(&self) -> Result<(), EndpointError> {
        if !valid_contract_id(&self.protocol)
            || !valid_contract_id(&self.binding)
            || self.endpoint.is_empty()
            || self.endpoint.len() > 512
            || self.protocol_versions.is_empty()
            || self.protocol_versions.len() > 16
        {
            return Err(EndpointError::DiscoveryUnavailable);
        }
        let mut unique_versions = BTreeSet::new();
        for version in &self.protocol_versions {
            if !valid_contract_version(version) || !unique_versions.insert(version) {
                return Err(EndpointError::DiscoveryUnavailable);
            }
        }
        Ok(())
    }
}

fn valid_service_kind(value: &str) -> bool {
    if value.is_empty() || value.len() > 64 {
        return false;
    }
    let mut segments = value.split(['.', '-']);
    segments.next().is_some_and(|segment| {
        segment
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_lowercase())
            && segment
                .chars()
                .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
    }) && segments.all(|segment| {
        !segment.is_empty()
            && segment
                .chars()
                .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
    })
}

fn valid_file_token(value: &str) -> bool {
    valid_token(value, 96, "._-")
}

fn valid_contract_id(value: &str) -> bool {
    valid_token(value, 128, "._:+/@%-")
}

fn valid_contract_version(value: &str) -> bool {
    valid_token(value, 64, "._:+-")
}

fn valid_token(value: &str, max_len: usize, punctuation: &str) -> bool {
    !value.is_empty()
        && value.len() <= max_len
        && value
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphanumeric())
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || punctuation.contains(character))
}

#[cfg(test)]
mod tests;
