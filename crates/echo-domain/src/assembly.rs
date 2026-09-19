//! User-authored multi-asset sound assembly documents.
//!
//! This owner is deliberately separate from [`crate::AdjustmentGraph`]. An
//! adjustment owns one immutable Original's source-time processing; an
//! assembly owns placement and mixing in a new composition-time coordinate
//! system. Clips reference an exact asset adjustment revision whose linear
//! render becomes their source without mutating that asset or revision.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{
    AssemblyClipId, AssemblyTrackId, AssetId, FadeCurve, MAX_GAIN_CENTIBELS, MIN_GAIN_CENTIBELS,
    SoundAssemblyId,
};

pub const MAX_ASSEMBLY_TRACKS: usize = 8;
pub const MAX_ASSEMBLY_CLIPS: usize = 256;
// Four hours keeps a 48 kHz stereo PCM24 mixdown within classic RIFF/WAVE's
// 32-bit data-chunk limit. RF64 can raise this contract in a later milestone.
pub const MAX_ASSEMBLY_DURATION_MILLIS: u64 = 4 * 60 * 60 * 1_000;
pub const MAX_ASSEMBLY_NAME_CHARACTERS: usize = 120;
pub const MAX_ASSEMBLY_TRACK_NAME_CHARACTERS: usize = 80;
pub const MIN_ASSEMBLY_PAN_PERCENT: i8 = -100;
pub const MAX_ASSEMBLY_PAN_PERCENT: i8 = 100;

/// The user's intent for a clip reference, independent of its track or later collection changes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssemblySourceRole {
    #[default]
    Memory,
    Material,
}

/// One clip sourced from the linear render of an exact asset revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssemblyClip {
    id: AssemblyClipId,
    asset_id: AssetId,
    adjustment_revision_id: i64,
    #[serde(default)]
    source_role: AssemblySourceRole,
    source_start_millis: u64,
    source_end_millis: u64,
    timeline_start_millis: u64,
    gain_centibels: i16,
    pan_percent: i8,
    fade_in_millis: u64,
    fade_out_millis: u64,
    fade_in_curve: FadeCurve,
    fade_out_curve: FadeCurve,
    muted: bool,
}

impl AssemblyClip {
    /// Creates a source-revision-pinned clip in composition time.
    ///
    /// # Errors
    ///
    /// Returns [`SoundAssemblyError`] when the source range, placement, fades,
    /// gain, pan, or adjustment revision is outside the bounded contract.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: AssemblyClipId,
        asset_id: AssetId,
        adjustment_revision_id: i64,
        source_start_millis: u64,
        source_end_millis: u64,
        timeline_start_millis: u64,
        gain_centibels: i16,
        pan_percent: i8,
        fade_in_millis: u64,
        fade_out_millis: u64,
        fade_in_curve: FadeCurve,
        fade_out_curve: FadeCurve,
        muted: bool,
    ) -> Result<Self, SoundAssemblyError> {
        if adjustment_revision_id < 0 {
            return Err(SoundAssemblyError::InvalidAdjustmentRevision);
        }
        if source_start_millis >= source_end_millis {
            return Err(SoundAssemblyError::InvalidClipSourceRange);
        }
        let duration = source_end_millis - source_start_millis;
        if fade_in_millis.saturating_add(fade_out_millis) > duration {
            return Err(SoundAssemblyError::OverlappingClipFades);
        }
        if !(MIN_GAIN_CENTIBELS..=MAX_GAIN_CENTIBELS).contains(&gain_centibels) {
            return Err(SoundAssemblyError::GainOutOfRange);
        }
        if !(MIN_ASSEMBLY_PAN_PERCENT..=MAX_ASSEMBLY_PAN_PERCENT).contains(&pan_percent) {
            return Err(SoundAssemblyError::PanOutOfRange);
        }
        let timeline_end = timeline_start_millis
            .checked_add(duration)
            .ok_or(SoundAssemblyError::DurationOutOfRange)?;
        if timeline_end > MAX_ASSEMBLY_DURATION_MILLIS {
            return Err(SoundAssemblyError::DurationOutOfRange);
        }
        Ok(Self {
            id,
            asset_id,
            adjustment_revision_id,
            source_role: AssemblySourceRole::Memory,
            source_start_millis,
            source_end_millis,
            timeline_start_millis,
            gain_centibels,
            pan_percent,
            fade_in_millis,
            fade_out_millis,
            fade_in_curve,
            fade_out_curve,
            muted,
        })
    }

    /// Selects the meaning of this reference without changing its immutable source.
    #[must_use]
    pub const fn with_source_role(mut self, role: AssemblySourceRole) -> Self {
        self.source_role = role;
        self
    }

    #[must_use]
    pub const fn source_role(&self) -> AssemblySourceRole {
        self.source_role
    }

    #[must_use]
    pub const fn id(&self) -> AssemblyClipId {
        self.id
    }

    #[must_use]
    pub const fn asset_id(&self) -> AssetId {
        self.asset_id
    }

    #[must_use]
    pub const fn adjustment_revision_id(&self) -> i64 {
        self.adjustment_revision_id
    }

    #[must_use]
    pub const fn source_start_millis(&self) -> u64 {
        self.source_start_millis
    }

    #[must_use]
    pub const fn source_end_millis(&self) -> u64 {
        self.source_end_millis
    }

    #[must_use]
    pub const fn timeline_start_millis(&self) -> u64 {
        self.timeline_start_millis
    }

    #[must_use]
    pub const fn duration_millis(&self) -> u64 {
        self.source_end_millis - self.source_start_millis
    }

    #[must_use]
    pub const fn timeline_end_millis(&self) -> u64 {
        self.timeline_start_millis + self.duration_millis()
    }

    #[must_use]
    pub const fn gain_centibels(&self) -> i16 {
        self.gain_centibels
    }

    #[must_use]
    pub const fn pan_percent(&self) -> i8 {
        self.pan_percent
    }

    #[must_use]
    pub const fn fade_in_millis(&self) -> u64 {
        self.fade_in_millis
    }

    #[must_use]
    pub const fn fade_out_millis(&self) -> u64 {
        self.fade_out_millis
    }

    #[must_use]
    pub const fn fade_in_curve(&self) -> FadeCurve {
        self.fade_in_curve
    }

    #[must_use]
    pub const fn fade_out_curve(&self) -> FadeCurve {
        self.fade_out_curve
    }

    #[must_use]
    pub const fn muted(&self) -> bool {
        self.muted
    }

    /// Re-applies the constructor contract after a serialization boundary.
    ///
    /// # Errors
    ///
    /// Returns the violated clip contract.
    pub fn validate(&self) -> Result<(), SoundAssemblyError> {
        Self::new(
            self.id,
            self.asset_id,
            self.adjustment_revision_id,
            self.source_start_millis,
            self.source_end_millis,
            self.timeline_start_millis,
            self.gain_centibels,
            self.pan_percent,
            self.fade_in_millis,
            self.fade_out_millis,
            self.fade_in_curve,
            self.fade_out_curve,
            self.muted,
        )
        .map(|_| ())
    }
}

/// One ordered audio track and its clips.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssemblyTrack {
    id: AssemblyTrackId,
    name: String,
    gain_centibels: i16,
    pan_percent: i8,
    muted: bool,
    solo: bool,
    clips: Vec<AssemblyClip>,
}

impl AssemblyTrack {
    /// Creates one ordered track with bounded mix controls and clip identities.
    ///
    /// # Errors
    ///
    /// Returns [`SoundAssemblyError`] for an invalid name, mix value, clip
    /// count, or duplicate clip identity.
    pub fn new(
        id: AssemblyTrackId,
        name: String,
        gain_centibels: i16,
        pan_percent: i8,
        muted: bool,
        solo: bool,
        clips: Vec<AssemblyClip>,
    ) -> Result<Self, SoundAssemblyError> {
        validate_name(
            &name,
            MAX_ASSEMBLY_TRACK_NAME_CHARACTERS,
            SoundAssemblyError::InvalidTrackName,
        )?;
        if !(MIN_GAIN_CENTIBELS..=MAX_GAIN_CENTIBELS).contains(&gain_centibels) {
            return Err(SoundAssemblyError::GainOutOfRange);
        }
        if !(MIN_ASSEMBLY_PAN_PERCENT..=MAX_ASSEMBLY_PAN_PERCENT).contains(&pan_percent) {
            return Err(SoundAssemblyError::PanOutOfRange);
        }
        if clips.len() > MAX_ASSEMBLY_CLIPS {
            return Err(SoundAssemblyError::InvalidClipCount);
        }
        let mut ids = BTreeSet::new();
        if clips.iter().any(|clip| !ids.insert(clip.id())) {
            return Err(SoundAssemblyError::DuplicateClipIdentity);
        }
        Ok(Self {
            id,
            name,
            gain_centibels,
            pan_percent,
            muted,
            solo,
            clips,
        })
    }

    #[must_use]
    pub const fn id(&self) -> AssemblyTrackId {
        self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn gain_centibels(&self) -> i16 {
        self.gain_centibels
    }

    #[must_use]
    pub const fn pan_percent(&self) -> i8 {
        self.pan_percent
    }

    #[must_use]
    pub const fn muted(&self) -> bool {
        self.muted
    }

    #[must_use]
    pub const fn solo(&self) -> bool {
        self.solo
    }

    #[must_use]
    pub fn clips(&self) -> &[AssemblyClip] {
        &self.clips
    }

    /// Re-applies the complete track and nested clip contract after decode.
    ///
    /// # Errors
    ///
    /// Returns the first violated track or nested clip contract.
    pub fn validate(&self) -> Result<(), SoundAssemblyError> {
        for clip in &self.clips {
            clip.validate()?;
        }
        Self::new(
            self.id,
            self.name.clone(),
            self.gain_centibels,
            self.pan_percent,
            self.muted,
            self.solo,
            self.clips.clone(),
        )
        .map(|_| ())
    }
}

/// Authored master output intent for one assembly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssemblyMaster {
    gain_centibels: i16,
    limiter_enabled: bool,
    limiter_ceiling_centibels: i16,
    limiter_release_millis: u16,
}

impl AssemblyMaster {
    /// Creates a bounded master-output configuration.
    ///
    /// # Errors
    ///
    /// Returns [`SoundAssemblyError`] for unsupported gain or limiter values.
    pub fn new(
        gain_centibels: i16,
        limiter_enabled: bool,
        limiter_ceiling_centibels: i16,
        limiter_release_millis: u16,
    ) -> Result<Self, SoundAssemblyError> {
        if !(MIN_GAIN_CENTIBELS..=MAX_GAIN_CENTIBELS).contains(&gain_centibels) {
            return Err(SoundAssemblyError::GainOutOfRange);
        }
        if !(-600..=0).contains(&limiter_ceiling_centibels)
            || !(20..=1_000).contains(&limiter_release_millis)
        {
            return Err(SoundAssemblyError::InvalidLimiter);
        }
        Ok(Self {
            gain_centibels,
            limiter_enabled,
            limiter_ceiling_centibels,
            limiter_release_millis,
        })
    }

    #[must_use]
    pub const fn standard() -> Self {
        Self {
            gain_centibels: 0,
            limiter_enabled: true,
            limiter_ceiling_centibels: -100,
            limiter_release_millis: 100,
        }
    }

    #[must_use]
    pub const fn gain_centibels(self) -> i16 {
        self.gain_centibels
    }

    #[must_use]
    pub const fn limiter_enabled(self) -> bool {
        self.limiter_enabled
    }

    #[must_use]
    pub const fn limiter_ceiling_centibels(self) -> i16 {
        self.limiter_ceiling_centibels
    }

    #[must_use]
    pub const fn limiter_release_millis(self) -> u16 {
        self.limiter_release_millis
    }

    /// Re-applies the master-output contract after a serialization boundary.
    ///
    /// # Errors
    ///
    /// Returns the violated master-output contract.
    pub fn validate(self) -> Result<(), SoundAssemblyError> {
        Self::new(
            self.gain_centibels,
            self.limiter_enabled,
            self.limiter_ceiling_centibels,
            self.limiter_release_millis,
        )
        .map(|_| ())
    }
}

/// One validated, user-authored assembly document snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoundAssembly {
    id: SoundAssemblyId,
    name: String,
    master: AssemblyMaster,
    tracks: Vec<AssemblyTrack>,
}

impl SoundAssembly {
    /// Creates one complete, bounded multi-asset assembly document.
    ///
    /// # Errors
    ///
    /// Returns [`SoundAssemblyError`] for invalid document naming, track or
    /// clip counts, or duplicate identities.
    pub fn new(
        id: SoundAssemblyId,
        name: String,
        master: AssemblyMaster,
        tracks: Vec<AssemblyTrack>,
    ) -> Result<Self, SoundAssemblyError> {
        validate_name(
            &name,
            MAX_ASSEMBLY_NAME_CHARACTERS,
            SoundAssemblyError::InvalidName,
        )?;
        if tracks.is_empty() || tracks.len() > MAX_ASSEMBLY_TRACKS {
            return Err(SoundAssemblyError::InvalidTrackCount);
        }
        let mut track_ids = BTreeSet::new();
        let mut clip_ids = BTreeSet::new();
        let mut clip_count = 0_usize;
        for track in &tracks {
            if !track_ids.insert(track.id()) {
                return Err(SoundAssemblyError::DuplicateTrackIdentity);
            }
            clip_count = clip_count
                .checked_add(track.clips().len())
                .ok_or(SoundAssemblyError::InvalidClipCount)?;
            if track.clips().iter().any(|clip| !clip_ids.insert(clip.id())) {
                return Err(SoundAssemblyError::DuplicateClipIdentity);
            }
        }
        if clip_count == 0 || clip_count > MAX_ASSEMBLY_CLIPS {
            return Err(SoundAssemblyError::InvalidClipCount);
        }
        Ok(Self {
            id,
            name,
            master,
            tracks,
        })
    }

    #[must_use]
    pub const fn id(&self) -> SoundAssemblyId {
        self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn master(&self) -> AssemblyMaster {
        self.master
    }

    #[must_use]
    pub fn tracks(&self) -> &[AssemblyTrack] {
        &self.tracks
    }

    #[must_use]
    pub fn clip_count(&self) -> usize {
        self.tracks.iter().map(|track| track.clips().len()).sum()
    }

    #[must_use]
    pub fn duration_millis(&self) -> u64 {
        self.tracks
            .iter()
            .flat_map(AssemblyTrack::clips)
            .map(AssemblyClip::timeline_end_millis)
            .max()
            .unwrap_or(0)
    }

    /// Re-applies every document invariant after a serialization boundary.
    ///
    /// # Errors
    ///
    /// Returns the first violated document, master, track, or clip contract.
    pub fn validate(&self) -> Result<(), SoundAssemblyError> {
        self.master.validate()?;
        for track in &self.tracks {
            track.validate()?;
        }
        Self::new(self.id, self.name.clone(), self.master, self.tracks.clone()).map(|_| ())
    }
}

fn validate_name(
    value: &str,
    maximum_characters: usize,
    error: SoundAssemblyError,
) -> Result<(), SoundAssemblyError> {
    if value.trim().is_empty() || value.chars().count() > maximum_characters {
        return Err(error);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundAssemblyError {
    InvalidName,
    InvalidTrackCount,
    InvalidTrackName,
    InvalidClipCount,
    DuplicateTrackIdentity,
    DuplicateClipIdentity,
    InvalidAdjustmentRevision,
    InvalidClipSourceRange,
    OverlappingClipFades,
    GainOutOfRange,
    PanOutOfRange,
    DurationOutOfRange,
    InvalidLimiter,
}

impl std::fmt::Display for SoundAssemblyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidName => "assembly name must be non-empty and at most 120 characters",
            Self::InvalidTrackCount => "assembly must contain between 1 and 8 tracks",
            Self::InvalidTrackName => "track name must be non-empty and at most 80 characters",
            Self::InvalidClipCount => "assembly must contain between 1 and 256 clips",
            Self::DuplicateTrackIdentity => "assembly track identities must be unique",
            Self::DuplicateClipIdentity => "assembly clip identities must be unique",
            Self::InvalidAdjustmentRevision => "clip adjustment revision must be zero or positive",
            Self::InvalidClipSourceRange => "clip source range must be non-empty",
            Self::OverlappingClipFades => "clip fades must fit inside the clip source range",
            Self::GainOutOfRange => "assembly gain must be between -24 dB and +12 dB",
            Self::PanOutOfRange => "assembly pan must be between -100 and 100 percent",
            Self::DurationOutOfRange => "assembly duration must not exceed 4 hours",
            Self::InvalidLimiter => "assembly limiter is outside the supported range",
        })
    }
}

impl std::error::Error for SoundAssemblyError {}

#[cfg(test)]
mod tests;
