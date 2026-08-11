//! Source-anchored non-destructive edit regions and effect scopes.
//!
//! This owner preserves the immutable original-time relationship. It validates
//! source-order coverage, recoverable visibility state, bounded inserted
//! silence, and structurally valid effect masks. [`crate::AdjustmentGraph`]
//! owns the final cross-check against one asset's trim and active effect chain.

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};

use crate::adjustment::{
    EFFECT_NODE_COUNT, EffectNodeKind, FadeCurve, FadeCurves, MAX_GAIN_CENTIBELS,
    MIN_GAIN_CENTIBELS,
};

/// Maximum number of source-ordered regions in one asset-local edit timeline.
pub const MAX_EDIT_SEGMENTS: usize = 128;
/// Maximum inserted silence after one source segment.
pub const MAX_EDIT_GAP_MILLIS: u64 = 3_600_000;
/// Maximum number of asset-local effect masks in one adjustment graph.
pub const MAX_EFFECT_MASKS: usize = 64;
/// Maximum source-time feather softened inside each effect-mask boundary.
pub const MAX_EFFECT_MASK_FEATHER_MILLIS: u16 = 100;
/// Default source-time feather for a newly authored effect mask.
pub const DEFAULT_EFFECT_MASK_FEATHER_MILLIS: u16 = 10;

/// Whether one source-anchored edit segment contributes audible samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EditSegmentState {
    Audible = 0,
    Muted = 1,
    Hidden = 2,
}

impl EditSegmentState {
    #[must_use]
    pub const fn wire_value(self) -> u8 {
        self as u8
    }

    /// Restores the stable numeric wire identity.
    ///
    /// # Errors
    ///
    /// Returns [`EditSegmentStateValueError`] for values outside `0..=2`.
    pub const fn from_wire_value(value: u8) -> Result<Self, EditSegmentStateValueError> {
        match value {
            0 => Ok(Self::Audible),
            1 => Ok(Self::Muted),
            2 => Ok(Self::Hidden),
            _ => Err(EditSegmentStateValueError),
        }
    }
}

impl Serialize for EditSegmentState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u8(self.wire_value())
    }
}

impl<'de> Deserialize<'de> for EditSegmentState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::from_wire_value(u8::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditSegmentStateValueError;

impl std::fmt::Display for EditSegmentStateValueError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("edit segment state must be audible, muted, or hidden")
    }
}

impl std::error::Error for EditSegmentStateValueError {}

/// One immutable-original interval in authored source order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditSegment {
    source_start_millis: u64,
    source_end_millis: u64,
    state: EditSegmentState,
    gain_centibels: i16,
    fade_in_millis: u64,
    fade_out_millis: u64,
    fade_in_curve: FadeCurve,
    fade_out_curve: FadeCurve,
    gap_after_millis: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredEditSegment {
    source_start_millis: u64,
    source_end_millis: u64,
    state: EditSegmentState,
    gain_centibels: i16,
    fade_in_millis: u64,
    fade_out_millis: u64,
    fade_in_curve: FadeCurve,
    fade_out_curve: FadeCurve,
    gap_after_millis: u64,
}

impl<'de> Deserialize<'de> for EditSegment {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let stored = StoredEditSegment::deserialize(deserializer)?;
        Self::new(
            stored.source_start_millis,
            stored.source_end_millis,
            stored.state,
            stored.gain_centibels,
            stored.fade_in_millis,
            stored.fade_out_millis,
            FadeCurves::new(stored.fade_in_curve, stored.fade_out_curve),
            stored.gap_after_millis,
        )
        .map_err(D::Error::custom)
    }
}

impl EditSegment {
    /// Creates one validated source interval.
    ///
    /// # Errors
    ///
    /// Returns [`EditTimelineError`] when the interval, gain, fades, or gap is
    /// outside Echo's bounded source-edit contract.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source_start_millis: u64,
        source_end_millis: u64,
        state: EditSegmentState,
        gain_centibels: i16,
        fade_in_millis: u64,
        fade_out_millis: u64,
        fade_curves: FadeCurves,
        gap_after_millis: u64,
    ) -> Result<Self, EditTimelineError> {
        if source_start_millis >= source_end_millis {
            return Err(EditTimelineError::InvalidSegmentRange);
        }
        if !(MIN_GAIN_CENTIBELS..=MAX_GAIN_CENTIBELS).contains(&gain_centibels) {
            return Err(EditTimelineError::GainOutOfRange);
        }
        if fade_in_millis.saturating_add(fade_out_millis) > source_end_millis - source_start_millis
        {
            return Err(EditTimelineError::OverlappingFades);
        }
        if gap_after_millis > MAX_EDIT_GAP_MILLIS {
            return Err(EditTimelineError::GapOutOfRange);
        }
        Ok(Self {
            source_start_millis,
            source_end_millis,
            state,
            gain_centibels,
            fade_in_millis,
            fade_out_millis,
            fade_in_curve: fade_curves.fade_in,
            fade_out_curve: fade_curves.fade_out,
            gap_after_millis,
        })
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
    pub const fn state(&self) -> EditSegmentState {
        self.state
    }

    #[must_use]
    pub const fn gain_centibels(&self) -> i16 {
        self.gain_centibels
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
    pub const fn gap_after_millis(&self) -> u64 {
        self.gap_after_millis
    }
}

/// A bounded source-ordered edit decision list covering the full trim range.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditTimeline {
    trim_start_millis: u64,
    trim_end_millis: u64,
    segments: Vec<EditSegment>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredEditTimeline {
    trim_start_millis: u64,
    trim_end_millis: u64,
    segments: Vec<EditSegment>,
}

impl<'de> Deserialize<'de> for EditTimeline {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let stored = StoredEditTimeline::deserialize(deserializer)?;
        Self::new(
            stored.trim_start_millis,
            stored.trim_end_millis,
            stored.segments,
        )
        .map_err(D::Error::custom)
    }
}

impl EditTimeline {
    /// Creates a continuous, source-ordered timeline covering the trim range.
    ///
    /// # Errors
    ///
    /// Returns [`EditTimelineError`] for missing, discontinuous, excessive, or
    /// empty-output segment lists.
    pub fn new(
        trim_start_millis: u64,
        trim_end_millis: u64,
        segments: Vec<EditSegment>,
    ) -> Result<Self, EditTimelineError> {
        if trim_start_millis >= trim_end_millis {
            return Err(EditTimelineError::InvalidTrimRange);
        }
        if segments.is_empty() || segments.len() > MAX_EDIT_SEGMENTS {
            return Err(EditTimelineError::InvalidSegmentCount);
        }
        if segments.first().map(EditSegment::source_start_millis) != Some(trim_start_millis)
            || segments.last().map(EditSegment::source_end_millis) != Some(trim_end_millis)
            || segments
                .windows(2)
                .any(|pair| pair[0].source_end_millis() != pair[1].source_start_millis())
        {
            return Err(EditTimelineError::DiscontinuousSegments);
        }
        let timeline = Self {
            trim_start_millis,
            trim_end_millis,
            segments,
        };
        if timeline.output_duration_millis() == 0 {
            return Err(EditTimelineError::EmptyOutput);
        }
        Ok(timeline)
    }

    /// Creates the one-segment identity timeline for a trim range.
    ///
    /// # Errors
    ///
    /// Returns [`EditTimelineError::InvalidTrimRange`] for an empty range.
    pub fn identity(
        trim_start_millis: u64,
        trim_end_millis: u64,
    ) -> Result<Self, EditTimelineError> {
        Self::new(
            trim_start_millis,
            trim_end_millis,
            vec![EditSegment::new(
                trim_start_millis,
                trim_end_millis,
                EditSegmentState::Audible,
                0,
                0,
                0,
                FadeCurves::linear(),
                0,
            )?],
        )
    }

    #[must_use]
    pub const fn trim_start_millis(&self) -> u64 {
        self.trim_start_millis
    }

    #[must_use]
    pub const fn trim_end_millis(&self) -> u64 {
        self.trim_end_millis
    }

    #[must_use]
    pub fn segments(&self) -> &[EditSegment] {
        &self.segments
    }

    /// Duration of the authored result, including muted source time and gaps.
    #[must_use]
    pub fn output_duration_millis(&self) -> u64 {
        self.segments.iter().fold(0_u64, |duration, segment| {
            let source_duration = if segment.state() == EditSegmentState::Hidden {
                0
            } else {
                segment.source_end_millis() - segment.source_start_millis()
            };
            duration
                .saturating_add(source_duration)
                .saturating_add(segment.gap_after_millis())
        })
    }
}

/// Stable validation failures for one source edit timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditTimelineError {
    InvalidTrimRange,
    InvalidSegmentCount,
    InvalidSegmentRange,
    DiscontinuousSegments,
    GainOutOfRange,
    OverlappingFades,
    GapOutOfRange,
    EmptyOutput,
}

impl std::fmt::Display for EditTimelineError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidTrimRange => "edit timeline trim range must be non-empty",
            Self::InvalidSegmentCount => "edit timeline must contain between 1 and 128 segments",
            Self::InvalidSegmentRange => "edit segment source range must be non-empty",
            Self::DiscontinuousSegments => {
                "edit segments must continuously cover the complete trim range"
            }
            Self::GainOutOfRange => "edit segment gain must be between -24 dB and +12 dB",
            Self::OverlappingFades => "edit segment fades must fit inside the source interval",
            Self::GapOutOfRange => "edit segment gap must not exceed one hour",
            Self::EmptyOutput => "edit timeline result must be non-empty",
        })
    }
}

impl std::error::Error for EditTimelineError {}

/// One original-time mask limiting selected insert effects to a source range.
///
/// When a node appears in any mask, it is active only in that node's authored
/// mask union. Overlapping masks preserve the authored effect-chain order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectMask {
    start_millis: u64,
    end_millis: u64,
    feather_millis: u16,
    effect_nodes: Vec<EffectNodeKind>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredEffectMask {
    start_millis: u64,
    end_millis: u64,
    #[serde(default = "default_effect_mask_feather_millis")]
    feather_millis: u16,
    effect_nodes: Vec<EffectNodeKind>,
}

const fn default_effect_mask_feather_millis() -> u16 {
    DEFAULT_EFFECT_MASK_FEATHER_MILLIS
}

impl<'de> Deserialize<'de> for EffectMask {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let stored = StoredEffectMask::deserialize(deserializer)?;
        Self::new(
            stored.start_millis,
            stored.end_millis,
            stored.feather_millis,
            stored.effect_nodes,
        )
        .map_err(D::Error::custom)
    }
}

impl EffectMask {
    /// Creates one structurally valid source-time mask.
    ///
    /// Active-chain membership and trim containment are validated when the
    /// mask is attached to an [`crate::AdjustmentGraph`].
    ///
    /// # Errors
    ///
    /// Returns [`EffectMaskError`] for an empty range, invalid feather, empty
    /// or duplicate node set, or unsupported Master/DeClick targeting.
    pub fn new(
        start_millis: u64,
        end_millis: u64,
        feather_millis: u16,
        effect_nodes: Vec<EffectNodeKind>,
    ) -> Result<Self, EffectMaskError> {
        if start_millis >= end_millis {
            return Err(EffectMaskError::InvalidRange);
        }
        if feather_millis > MAX_EFFECT_MASK_FEATHER_MILLIS {
            return Err(EffectMaskError::FeatherOutOfRange);
        }
        if effect_nodes.is_empty() {
            return Err(EffectMaskError::EmptyEffectNodes);
        }
        let mut seen = [false; EFFECT_NODE_COUNT];
        for node in &effect_nodes {
            if matches!(node, EffectNodeKind::Master | EffectNodeKind::DeClick) {
                return Err(EffectMaskError::UnsupportedEffectNode);
            }
            let index = *node as usize;
            if seen[index] {
                return Err(EffectMaskError::DuplicateEffectNode);
            }
            seen[index] = true;
        }
        Ok(Self {
            start_millis,
            end_millis,
            feather_millis,
            effect_nodes,
        })
    }

    #[must_use]
    pub const fn start_millis(&self) -> u64 {
        self.start_millis
    }

    #[must_use]
    pub const fn end_millis(&self) -> u64 {
        self.end_millis
    }

    #[must_use]
    pub const fn feather_millis(&self) -> u16 {
        self.feather_millis
    }

    #[must_use]
    pub fn effect_nodes(&self) -> &[EffectNodeKind] {
        &self.effect_nodes
    }
}

/// Structural validation failures for one effect mask.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectMaskError {
    InvalidRange,
    FeatherOutOfRange,
    EmptyEffectNodes,
    DuplicateEffectNode,
    UnsupportedEffectNode,
}

impl std::fmt::Display for EffectMaskError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidRange => "effect mask range must be non-empty",
            Self::FeatherOutOfRange => "effect mask feather must be between 0 and 100 ms",
            Self::EmptyEffectNodes => "effect mask must target at least one insert effect",
            Self::DuplicateEffectNode => "effect mask effect nodes must be unique",
            Self::UnsupportedEffectNode => {
                "effect masks cannot target master output or fixed-latency de-click"
            }
        })
    }
}

impl std::error::Error for EffectMaskError {}

#[cfg(test)]
mod tests;
