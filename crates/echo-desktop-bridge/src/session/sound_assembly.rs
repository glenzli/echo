//! Desktop-facing orchestration for Sound Assembly documents.
//!
//! This owner translates bounded Library selections into domain documents and
//! resolves exact adjustment revisions for the native preparation pipeline.
//! It does not render audio or expose `SQLite` details to Qt.

use std::{collections::BTreeSet, path::Path, str::FromStr};

use echo_catalog::{
    AssetLookup, RecordSoundAssemblyExport, SoundAssemblyExportFormat, SoundAssemblyRevision,
    adjustment_graph_at_revision, archive_sound_assembly, find_by_id, latest_adjustment_graph,
    latest_sound_assembly, list_sound_assemblies, record_sound_assembly,
    record_sound_assembly_export,
};
use echo_domain::{
    AssemblyClip, AssemblyClipId, AssemblyMaster, AssemblySourceRole, AssemblyTrack,
    AssemblyTrackId, AssetId, FadeCurve, SoundAssembly, SoundAssemblyId,
};

use super::{
    LibrarySession, SessionError, adjustment_wire_fields, asset_adjustment_wire, now_millis,
};
use crate::ffi::{
    RenderExportWire, SoundAssemblyClipSourceWire, SoundAssemblyRevisionWire,
    SoundAssemblySummaryWire,
};

impl LibrarySession {
    pub(crate) fn sound_assemblies(&self) -> Result<Vec<SoundAssemblySummaryWire>, SessionError> {
        self.catalog
            .with_transaction(list_sound_assemblies)
            .map_err(SessionError::from)
            .map(|values| {
                values
                    .into_iter()
                    .map(|value| SoundAssemblySummaryWire {
                        assembly_id: value.assembly_id.to_string(),
                        name: value.name,
                        revision_id: value.revision_id,
                        revision_number: value.revision_number,
                        duration_millis: value.duration_millis,
                        track_count: u32::try_from(value.track_count).unwrap_or(u32::MAX),
                        clip_count: u32::try_from(value.clip_count).unwrap_or(u32::MAX),
                        updated_at_millis: value.updated_at_millis,
                    })
                    .collect()
            })
    }

    #[allow(clippy::too_many_lines)] // Resolve source versions and lay out the complete initial document together.
    pub(crate) fn create_sound_assembly(
        &self,
        name: &str,
        asset_ids: &[String],
        layout: u8,
    ) -> Result<SoundAssemblyRevisionWire, SessionError> {
        if asset_ids.is_empty() || asset_ids.len() > 256 {
            return Err(session_error(
                "assembly selection must contain between 1 and 256 assets",
            ));
        }
        if layout > 1 || (layout == 1 && asset_ids.len() > 8) {
            return Err(session_error(
                "layered assembly requires between 1 and 8 selected assets",
            ));
        }
        let mut seen = BTreeSet::new();
        let ids = asset_ids
            .iter()
            .map(|value| parse_asset_id(value))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .filter(|id| seen.insert(*id))
            .collect::<Vec<_>>();
        let sources = self.catalog.with_transaction(|transaction| {
            let memberships = echo_catalog::sound_memberships(transaction)?;
            ids.iter()
                .copied()
                .map(|asset_id| {
                    let asset = match find_by_id(transaction, asset_id)? {
                        AssetLookup::Found(asset) => asset,
                        AssetLookup::NotFound => {
                            return Err(echo_catalog::CatalogError::new(
                                echo_catalog::CatalogErrorKind::Other,
                                format!("assembly source asset {asset_id} does not exist"),
                            ));
                        }
                    };
                    let original_duration = asset.original.duration_millis.ok_or_else(|| {
                        echo_catalog::CatalogError::new(
                            echo_catalog::CatalogErrorKind::Other,
                            format!("assembly source asset {asset_id} has no known duration"),
                        )
                    })?;
                    let adjustment = latest_adjustment_graph(transaction, asset_id)?;
                    let revision_id = adjustment.as_ref().map_or(0, |value| value.revision_id);
                    let duration = adjustment.as_ref().map_or(original_duration, |value| {
                        value.graph.edit_timeline().output_duration_millis()
                    });
                    let display_name = asset
                        .original
                        .path
                        .file_stem()
                        .and_then(|value| value.to_str())
                        .filter(|value| !value.trim().is_empty())
                        .unwrap_or("Sound")
                        .to_owned();
                    let role = if memberships
                        .get(&asset_id.to_string())
                        .is_some_and(|value| value.in_memory)
                    {
                        AssemblySourceRole::Memory
                    } else {
                        AssemblySourceRole::Material
                    };
                    Ok((asset_id, revision_id, duration, display_name, role))
                })
                .collect::<Result<Vec<_>, echo_catalog::CatalogError>>()
        })?;
        let assembly_id = SoundAssemblyId::new();
        let tracks = if layout == 0 {
            let mut timeline_start = 0_u64;
            let clips = sources
                .iter()
                .map(|(asset_id, revision_id, duration, _, role)| {
                    let clip = assembly_clip(*asset_id, *revision_id, *duration, timeline_start)?
                        .with_source_role(*role);
                    timeline_start = timeline_start.checked_add(*duration).ok_or_else(|| {
                        session_error("assembly sequence duration exceeds the supported range")
                    })?;
                    Ok(clip)
                })
                .collect::<Result<Vec<_>, SessionError>>()?;
            vec![
                AssemblyTrack::new(
                    AssemblyTrackId::new(),
                    name.to_owned(),
                    0,
                    0,
                    false,
                    false,
                    clips,
                )
                .map_err(domain_error)?,
            ]
        } else {
            sources
                .iter()
                .map(|(asset_id, revision_id, duration, display_name, role)| {
                    AssemblyTrack::new(
                        AssemblyTrackId::new(),
                        display_name.clone(),
                        0,
                        0,
                        false,
                        false,
                        vec![
                            assembly_clip(*asset_id, *revision_id, *duration, 0)?
                                .with_source_role(*role),
                        ],
                    )
                    .map_err(domain_error)
                })
                .collect::<Result<Vec<_>, _>>()?
        };
        let document = SoundAssembly::new(
            assembly_id,
            name.to_owned(),
            AssemblyMaster::standard(),
            tracks,
        )
        .map_err(domain_error)?;
        let revision = self.catalog.with_transaction(|transaction| {
            record_sound_assembly(transaction, &document, now_millis())
        })?;
        self.resolve_sound_assembly_revision(&revision)
    }

    pub(crate) fn sound_assembly(
        &self,
        assembly_id: &str,
    ) -> Result<SoundAssemblyRevisionWire, SessionError> {
        let assembly_id = parse_assembly_id(assembly_id)?;
        let revision = self
            .catalog
            .with_transaction(|transaction| latest_sound_assembly(transaction, assembly_id))?
            .ok_or_else(|| session_error("sound assembly does not exist"))?;
        self.resolve_sound_assembly_revision(&revision)
    }

    pub(crate) fn save_sound_assembly(
        &self,
        document_json: &str,
    ) -> Result<SoundAssemblyRevisionWire, SessionError> {
        let document = serde_json::from_str::<SoundAssembly>(document_json)
            .map_err(|error| session_error(format!("assembly document is invalid: {error}")))?;
        document.validate().map_err(domain_error)?;
        let revision = self.catalog.with_transaction(|transaction| {
            record_sound_assembly(transaction, &document, now_millis())
        })?;
        self.resolve_sound_assembly_revision(&revision)
    }

    pub(crate) fn save_project_clip_adjustment(
        &self,
        document_json: &str,
        clip_id: &str,
        asset_id: &str,
        adjustment: &crate::ffi::AssetAdjustmentWire,
    ) -> Result<SoundAssemblyRevisionWire, SessionError> {
        let mut document: serde_json::Value =
            serde_json::from_str(document_json).map_err(|e| session_error(e.to_string()))?;
        let source_id = parse_asset_id(asset_id)?;
        let assembly_id = document
            .get("id")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| session_error("missing project identity"))?
            .to_owned();
        let assembly_id_typed = parse_assembly_id(&assembly_id)?;
        let revision = self
            .catalog
            .with_transaction(|tx| -> Result<_, SessionError> {
                if latest_sound_assembly(tx, assembly_id_typed)?.is_none() {
                    return Err(session_error("project does not exist"));
                }
                let asset = match find_by_id(tx, source_id)? {
                    AssetLookup::Found(a) => a,
                    AssetLookup::NotFound => return Err(session_error("source does not exist")),
                };
                let duration = asset
                    .original
                    .duration_millis
                    .ok_or_else(|| session_error("unknown source duration"))?;
                let graph = super::adjustment_graph_from_wire(duration, adjustment)?;
                let output_duration = graph.edit_timeline().output_duration_millis();
                let mut found = false;
                for track in document["tracks"]
                    .as_array_mut()
                    .ok_or_else(|| session_error("missing tracks"))?
                {
                    for clip in track["clips"]
                        .as_array_mut()
                        .ok_or_else(|| session_error("missing clips"))?
                    {
                        if clip["id"].as_str() != Some(clip_id) {
                            continue;
                        }
                        if clip["assetId"].as_str() != Some(asset_id) || output_duration == 0 {
                            return Err(session_error(
                                "project clip source does not match the editor",
                            ));
                        }
                        let old_revision = clip["adjustmentRevisionId"]
                            .as_i64()
                            .ok_or_else(|| session_error("missing source revision"))?;
                        let old_timeline = if old_revision == 0 {
                            echo_domain::EditTimeline::identity(0, duration)
                                .map_err(|e| session_error(e.to_string()))?
                        } else {
                            adjustment_graph_at_revision(tx, source_id, old_revision)?
                                .ok_or_else(|| session_error("missing source revision"))?
                                .graph
                                .edit_timeline()
                                .clone()
                        };
                        let old_duration = old_timeline.output_duration_millis();
                        if let Some(value) = clip.get("gainEnvelope") {
                            let envelope: echo_domain::GainEnvelope =
                                serde_json::from_value(value.clone())
                                    .map_err(|e| session_error(e.to_string()))?;
                            clip["gainEnvelope"] = serde_json::to_value(
                                envelope
                                    .remap_source_edits(&old_timeline, graph.edit_timeline())
                                    .map_err(|e| session_error(e.to_string()))?,
                            )
                            .map_err(|e| session_error(e.to_string()))?;
                        }
                        let saved = echo_catalog::record_project_adjustment_graph(
                            tx,
                            source_id,
                            &assembly_id,
                            clip_id,
                            graph.clone(),
                            now_millis(),
                        )?;
                        let old_start = clip["sourceStartMillis"]
                            .as_u64()
                            .ok_or_else(|| session_error("invalid clip range"))?;
                        let old_end = clip["sourceEndMillis"]
                            .as_u64()
                            .ok_or_else(|| session_error("invalid clip range"))?;
                        let start = old_start.min(output_duration - 1);
                        let end = if old_end == old_duration {
                            output_duration
                        } else {
                            old_end.min(output_duration)
                        };
                        let fade_in = clip["fadeInMillis"].as_u64().unwrap_or(0).min(end - start);
                        let fade_out = clip["fadeOutMillis"]
                            .as_u64()
                            .unwrap_or(0)
                            .min(end - start - fade_in);
                        clip["adjustmentRevisionId"] = saved.revision_id.into();
                        clip["sourceStartMillis"] = start.into();
                        clip["sourceEndMillis"] = end.into();
                        clip["fadeInMillis"] = fade_in.into();
                        clip["fadeOutMillis"] = fade_out.into();
                        found = true;
                    }
                }
                if !found {
                    return Err(session_error("project clip no longer exists"));
                }
                let authored: SoundAssembly =
                    serde_json::from_value(document).map_err(|e| session_error(e.to_string()))?;
                Ok(record_sound_assembly(tx, &authored, now_millis())?)
            })?;
        self.resolve_sound_assembly_revision(&revision)
    }

    pub(crate) fn archive_sound_assembly(&self, assembly_id: &str) -> Result<(), SessionError> {
        let assembly_id = parse_assembly_id(assembly_id)?;
        self.catalog
            .with_transaction(|transaction| {
                archive_sound_assembly(transaction, assembly_id, now_millis())
            })
            .map_err(SessionError::from)
    }

    pub(crate) fn record_sound_assembly_export(
        &self,
        assembly_id: &str,
        assembly_revision_id: i64,
        evidence: &RenderExportWire,
    ) -> Result<i64, SessionError> {
        let assembly_id = parse_assembly_id(assembly_id)?;
        if evidence.format != "wav_pcm24"
            || evidence.bit_depth != 24
            || evidence.sample_rate == 0
            || evidence.channel_count == 0
            || evidence.frame_count == 0
            || evidence.size_bytes == 0
        {
            return Err(session_error("assembly render evidence is invalid"));
        }
        let output_path = Path::new(&evidence.output_path);
        let metadata = output_path
            .metadata()
            .map_err(|error| session_error(format!("cannot inspect assembly mixdown: {error}")))?;
        if metadata.len() != evidence.size_bytes {
            return Err(session_error(
                "assembly mixdown size does not match render evidence",
            ));
        }
        let revision = self
            .catalog
            .with_transaction(|transaction| {
                echo_catalog::sound_assembly_at_revision(
                    transaction,
                    assembly_id,
                    assembly_revision_id,
                )
            })?
            .ok_or_else(|| session_error("assembly revision does not exist"))?;
        let canonical_output = output_path.canonicalize().map_err(|error| {
            session_error(format!("cannot resolve assembly mixdown path: {error}"))
        })?;
        for clip in revision
            .assembly
            .tracks()
            .iter()
            .flat_map(AssemblyTrack::clips)
        {
            let source = self.catalog.with_transaction(|transaction| {
                match find_by_id(transaction, clip.asset_id())? {
                    AssetLookup::Found(asset) => Ok(asset.original.path),
                    AssetLookup::NotFound => Err(echo_catalog::CatalogError::new(
                        echo_catalog::CatalogErrorKind::Other,
                        "assembly source asset disappeared",
                    )),
                }
            })?;
            if source
                .canonicalize()
                .is_ok_and(|path| path == canonical_output)
            {
                return Err(session_error(
                    "assembly mixdown cannot replace an immutable Original",
                ));
            }
        }
        let content_hash = echo_core::hash_file(output_path)
            .map_err(|error| session_error(format!("cannot hash assembly mixdown: {error}")))?;
        let record = self.catalog.with_transaction(|transaction| {
            Self::verify_export_disclosure(
                transaction,
                &assembly_id.to_string(),
                assembly_revision_id,
                &evidence.source_disclosure_comment,
            )?;
            record_sound_assembly_export(
                transaction,
                &RecordSoundAssemblyExport {
                    assembly_id,
                    assembly_revision_id,
                    output_path,
                    format: SoundAssemblyExportFormat::WavPcm24,
                    sample_rate: evidence.sample_rate,
                    channel_count: u16::try_from(evidence.channel_count).map_err(|_| {
                        echo_catalog::CatalogError::new(
                            echo_catalog::CatalogErrorKind::Other,
                            "assembly channel count exceeds the supported range",
                        )
                    })?,
                    frame_count: evidence.frame_count,
                    content_hash,
                    size_bytes: evidence.size_bytes,
                    integrated_lufs: f64::from(evidence.integrated_lufs),
                    true_peak_dbtp: f64::from(evidence.true_peak_dbtp),
                    created_at_millis: now_millis(),
                },
            )
        })?;
        Ok(record.export_id)
    }

    fn resolve_sound_assembly_revision(
        &self,
        revision: &SoundAssemblyRevision,
    ) -> Result<SoundAssemblyRevisionWire, SessionError> {
        let mut clip_sources = Vec::with_capacity(revision.assembly.clip_count());
        for clip in revision
            .assembly
            .tracks()
            .iter()
            .flat_map(AssemblyTrack::clips)
        {
            let (asset, adjustment) = self.catalog.with_transaction(|transaction| {
                let asset = match find_by_id(transaction, clip.asset_id())? {
                    AssetLookup::Found(asset) => asset,
                    AssetLookup::NotFound => {
                        return Err(echo_catalog::CatalogError::new(
                            echo_catalog::CatalogErrorKind::Other,
                            "assembly source asset does not exist",
                        ));
                    }
                };
                let adjustment = if clip.adjustment_revision_id() == 0 {
                    None
                } else {
                    Some(
                        adjustment_graph_at_revision(
                            transaction,
                            clip.asset_id(),
                            clip.adjustment_revision_id(),
                        )?
                        .ok_or_else(|| {
                            echo_catalog::CatalogError::new(
                                echo_catalog::CatalogErrorKind::Other,
                                "assembly source adjustment revision no longer resolves",
                            )
                        })?,
                    )
                };
                Ok((asset, adjustment))
            })?;
            let fields = adjustment_wire_fields(adjustment, asset.original.duration_millis);
            let prepared_path =
                echo_domain::ContentHash::from_str(&fields.impulse_response_prepared_hash)
                    .map(|hash| {
                        echo_cache::blob_path(&self.cache_root, &hash)
                            .to_string_lossy()
                            .into_owned()
                    })
                    .unwrap_or_default();
            clip_sources.push(SoundAssemblyClipSourceWire {
                clip_id: clip.id().to_string(),
                path: asset.original.path.to_string_lossy().into_owned(),
                adjustment_revision_id: fields.revision,
                impulse_response_prepared_path: prepared_path,
                adjustment: asset_adjustment_wire(fields),
            });
        }
        let document_json = serde_json::to_string(&revision.assembly)
            .map_err(|error| session_error(format!("assembly cannot be encoded: {error}")))?;
        Ok(SoundAssemblyRevisionWire {
            assembly_id: revision.assembly.id().to_string(),
            revision_id: revision.revision_id,
            revision_number: revision.revision_number,
            document_json,
            clip_sources,
            created_at_millis: revision.created_at_millis,
        })
    }
}

fn assembly_clip(
    asset_id: AssetId,
    revision_id: i64,
    duration_millis: u64,
    timeline_start_millis: u64,
) -> Result<AssemblyClip, SessionError> {
    AssemblyClip::new(
        AssemblyClipId::new(),
        asset_id,
        revision_id,
        0,
        duration_millis,
        timeline_start_millis,
        0,
        0,
        0,
        0,
        FadeCurve::Linear,
        FadeCurve::Linear,
        false,
    )
    .map_err(domain_error)
}

fn parse_asset_id(value: &str) -> Result<AssetId, SessionError> {
    AssetId::from_str(value)
        .map_err(|error| session_error(format!("invalid assembly asset id {value}: {error}")))
}

fn parse_assembly_id(value: &str) -> Result<SoundAssemblyId, SessionError> {
    SoundAssemblyId::from_str(value)
        .map_err(|error| session_error(format!("invalid assembly id {value}: {error}")))
}

fn domain_error(error: echo_domain::SoundAssemblyError) -> SessionError {
    session_error(error.to_string())
}

fn session_error(message: impl Into<String>) -> SessionError {
    SessionError {
        message: message.into(),
    }
}

#[cfg(test)]
mod tests;
