//! Reusable, non-destructive restoration intent.
//!
//! A processing recipe is an immutable snapshot of selected processing
//! components. Applying it always materializes a complete asset-local
//! [`AdjustmentGraph`]; playback and rendering never depend on this shared
//! definition. Clip-local trim, fades, and gain therefore remain owned by the
//! target asset and are never copied by a recipe. Source edit timelines and
//! effect masks are likewise asset-specific and always survive recipe apply.

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use crate::{
    AdjustmentEffects, AdjustmentGraph, AdjustmentGraphError, CompressorSettings, DeClickSettings,
    DeHumSettings, EffectChain, EffectNodeKind, FadeCurves, LimiterSettings, ParametricEqualizer,
    ProcessingRecipeId, ProcessingRecipeRevisionId, RestorationSettings, ReverbSettings,
};

/// Stable selectable processing component identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum ProcessingComponent {
    LowCut = 0,
    Restoration = 1,
    DeHum = 2,
    DeClick = 3,
    Equalizer = 4,
    Dynamics = 5,
    Space = 6,
    Master = 7,
}

/// Echo's complete reusable restoration-processing surface.
///
/// Clip-local trim, fades, and gain are intentionally absent.
pub const DEFAULT_PROCESSING_COMPONENTS: [ProcessingComponent; 8] = [
    ProcessingComponent::LowCut,
    ProcessingComponent::Restoration,
    ProcessingComponent::DeHum,
    ProcessingComponent::DeClick,
    ProcessingComponent::Equalizer,
    ProcessingComponent::Dynamics,
    ProcessingComponent::Space,
    ProcessingComponent::Master,
];

impl ProcessingComponent {
    #[must_use]
    pub const fn wire_value(self) -> u8 {
        self as u8
    }

    /// Restores the stable desktop wire representation.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessingComponentValueError`] for an unknown value.
    pub const fn from_wire_value(value: u8) -> Result<Self, ProcessingComponentValueError> {
        match value {
            0 => Ok(Self::LowCut),
            1 => Ok(Self::Restoration),
            2 => Ok(Self::DeHum),
            3 => Ok(Self::DeClick),
            4 => Ok(Self::Equalizer),
            5 => Ok(Self::Dynamics),
            6 => Ok(Self::Space),
            7 => Ok(Self::Master),
            _ => Err(ProcessingComponentValueError),
        }
    }

    const fn effect_node(self) -> Option<EffectNodeKind> {
        match self {
            Self::LowCut => None,
            Self::Restoration => Some(EffectNodeKind::Restoration),
            Self::DeHum => Some(EffectNodeKind::DeHum),
            Self::DeClick => Some(EffectNodeKind::DeClick),
            Self::Equalizer => Some(EffectNodeKind::Equalizer),
            Self::Dynamics => Some(EffectNodeKind::Dynamics),
            Self::Space => Some(EffectNodeKind::Space),
            Self::Master => Some(EffectNodeKind::Master),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessingComponentValueError;

impl std::fmt::Display for ProcessingComponentValueError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("processing component is outside the stable recipe contract")
    }
}

impl std::error::Error for ProcessingComponentValueError {}

/// How a processing patch is materialized into an asset-local graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessingMergeMode {
    #[serde(rename = "merge")]
    Merge,
    #[serde(rename = "replace_processing")]
    Replace,
}

/// Selected reusable processing from one validated adjustment graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AdjustmentPatch {
    components: Vec<ProcessingComponent>,
    low_cut_hertz: u16,
    restoration: RestorationSettings,
    de_hum: DeHumSettings,
    de_click: DeClickSettings,
    equalizer: ParametricEqualizer,
    compressor: CompressorSettings,
    reverb: ReverbSettings,
    limiter: LimiterSettings,
    effect_chain: EffectChain,
}

#[derive(Deserialize)]
struct StoredAdjustmentPatch {
    components: Vec<ProcessingComponent>,
    low_cut_hertz: u16,
    restoration: RestorationSettings,
    de_hum: DeHumSettings,
    de_click: DeClickSettings,
    equalizer: ParametricEqualizer,
    compressor: CompressorSettings,
    reverb: ReverbSettings,
    limiter: LimiterSettings,
    effect_chain: EffectChain,
}

impl<'de> Deserialize<'de> for AdjustmentPatch {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let stored = StoredAdjustmentPatch::deserialize(deserializer)?;
        let patch = Self {
            components: stored.components,
            low_cut_hertz: stored.low_cut_hertz,
            restoration: stored.restoration,
            de_hum: stored.de_hum,
            de_click: stored.de_click,
            equalizer: stored.equalizer,
            compressor: stored.compressor,
            reverb: stored.reverb,
            limiter: stored.limiter,
            effect_chain: stored.effect_chain,
        };
        patch.validate().map_err(D::Error::custom)?;
        Ok(patch)
    }
}

impl AdjustmentPatch {
    /// Captures selected reusable processing from a validated graph.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessingRecipeError`] when no component is selected or a
    /// component identity is repeated.
    #[allow(clippy::needless_pass_by_value)] // Stable API consumes one authored snapshot.
    pub fn from_graph(
        graph: AdjustmentGraph,
        components: &[ProcessingComponent],
    ) -> Result<Self, ProcessingRecipeError> {
        let patch = Self {
            components: components.to_vec(),
            low_cut_hertz: graph.low_cut_hertz(),
            restoration: graph.restoration(),
            de_hum: graph.de_hum(),
            de_click: graph.de_click(),
            equalizer: graph.equalizer(),
            compressor: graph.compressor(),
            reverb: graph.reverb(),
            limiter: graph.limiter(),
            effect_chain: graph.effect_chain(),
        };
        patch.validate()?;
        Ok(patch)
    }

    #[must_use]
    pub fn components(&self) -> &[ProcessingComponent] {
        &self.components
    }

    /// Materializes this patch into a new target-local graph.
    ///
    /// Both modes preserve target trim, fades, fade curves, and clip gain.
    /// Merge preserves unselected processing; replace resets it to defaults.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessingRecipeError`] when the patch or resulting graph is
    /// outside Echo's bounded adjustment contract.
    #[allow(clippy::needless_pass_by_value)] // Materialization consumes the target snapshot.
    pub fn apply_to(
        &self,
        target: AdjustmentGraph,
        mode: ProcessingMergeMode,
    ) -> Result<AdjustmentGraph, ProcessingRecipeError> {
        self.validate()?;
        let mut effects = match mode {
            ProcessingMergeMode::Merge => processing_from_graph(&target),
            ProcessingMergeMode::Replace => AdjustmentEffects::new(
                FadeCurves::new(target.fade_in_curve(), target.fade_out_curve()),
                target.gain_centibels(),
                0,
            )
            .with_effect_chain(EffectChain::new([EffectNodeKind::Master])?),
        };

        if self.contains(ProcessingComponent::LowCut) {
            effects.low_cut_hertz = self.low_cut_hertz;
        }
        if self.contains(ProcessingComponent::Restoration) {
            effects.restoration = self.restoration;
        }
        if self.contains(ProcessingComponent::DeHum) {
            effects.de_hum = self.de_hum;
        }
        if self.contains(ProcessingComponent::DeClick) {
            effects.de_click = self.de_click;
        }
        if self.contains(ProcessingComponent::Equalizer) {
            effects.equalizer = self.equalizer;
        }
        if self.contains(ProcessingComponent::Dynamics) {
            effects.compressor = self.compressor;
        }
        if self.contains(ProcessingComponent::Space) {
            effects.reverb = self.reverb;
        }
        if self.contains(ProcessingComponent::Master) {
            effects.limiter = self.limiter;
        }
        effects.effect_chain = match mode {
            ProcessingMergeMode::Merge => self.merged_chain(target.effect_chain())?,
            ProcessingMergeMode::Replace => self.replacement_chain()?,
        };
        effects = effects
            .with_edit_timeline(target.edit_timeline().clone())
            .with_effect_masks(target.effect_masks().to_vec());

        AdjustmentGraph::new(
            target.trim_end_millis(),
            target.trim_start_millis(),
            target.trim_end_millis(),
            target.fade_in_millis(),
            target.fade_out_millis(),
            effects,
        )
        .map_err(ProcessingRecipeError::InvalidAdjustment)
    }

    fn contains(&self, expected: ProcessingComponent) -> bool {
        self.components.contains(&expected)
    }

    fn validate(&self) -> Result<(), ProcessingRecipeError> {
        if self.components.is_empty() {
            return Err(ProcessingRecipeError::EmptyComponents);
        }
        let mut seen = [false; 8];
        for component in &self.components {
            let index = usize::from(component.wire_value());
            if seen[index] {
                return Err(ProcessingRecipeError::DuplicateComponent(*component));
            }
            seen[index] = true;
        }
        AdjustmentGraph::new(
            1,
            0,
            1,
            0,
            0,
            AdjustmentEffects::new(FadeCurves::linear(), 0, self.low_cut_hertz)
                .with_restoration(self.restoration)
                .with_de_hum(self.de_hum)
                .with_de_click(self.de_click)
                .with_equalizer(self.equalizer)
                .with_compressor(self.compressor)
                .with_reverb(self.reverb)
                .with_limiter(self.limiter)
                .with_effect_chain(self.effect_chain),
        )
        .map(|_| ())
        .map_err(ProcessingRecipeError::InvalidAdjustment)
    }

    fn replacement_chain(&self) -> Result<EffectChain, ProcessingRecipeError> {
        let mut nodes = Vec::with_capacity(8);
        for &node in self.effect_chain.nodes() {
            if node != EffectNodeKind::Master && self.selects_node(node) {
                nodes.push(node);
            }
        }
        nodes.push(EffectNodeKind::Master);
        EffectChain::from_active_nodes(&nodes).map_err(ProcessingRecipeError::from)
    }

    fn merged_chain(&self, target: EffectChain) -> Result<EffectChain, ProcessingRecipeError> {
        let selected_nodes = self
            .components
            .iter()
            .filter_map(|component| component.effect_node())
            .filter(|node| *node != EffectNodeKind::Master)
            .collect::<Vec<_>>();
        if selected_nodes.is_empty() {
            return Ok(target);
        }

        let mut retained = Vec::with_capacity(8);
        let mut insertion_index = None;
        for &node in target.nodes() {
            if node == EffectNodeKind::Master {
                continue;
            }
            if selected_nodes.contains(&node) {
                insertion_index.get_or_insert(retained.len());
            } else {
                retained.push(node);
            }
        }
        let insert_at = insertion_index.unwrap_or(retained.len());
        let selected_active = self
            .effect_chain
            .nodes()
            .iter()
            .copied()
            .filter(|node| *node != EffectNodeKind::Master && selected_nodes.contains(node))
            .collect::<Vec<_>>();
        retained.splice(insert_at..insert_at, selected_active);
        retained.push(EffectNodeKind::Master);
        EffectChain::from_active_nodes(&retained).map_err(ProcessingRecipeError::from)
    }

    fn selects_node(&self, node: EffectNodeKind) -> bool {
        self.components
            .iter()
            .any(|component| component.effect_node() == Some(node))
    }
}

fn processing_from_graph(graph: &AdjustmentGraph) -> AdjustmentEffects {
    AdjustmentEffects::new(
        FadeCurves::new(graph.fade_in_curve(), graph.fade_out_curve()),
        graph.gain_centibels(),
        graph.low_cut_hertz(),
    )
    .with_restoration(graph.restoration())
    .with_de_hum(graph.de_hum())
    .with_de_click(graph.de_click())
    .with_equalizer(graph.equalizer())
    .with_compressor(graph.compressor())
    .with_reverb(graph.reverb())
    .with_limiter(graph.limiter())
    .with_effect_chain(graph.effect_chain())
}

/// One immutable append-only processing recipe revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProcessingRecipeRevision {
    revision_id: ProcessingRecipeRevisionId,
    recipe_id: ProcessingRecipeId,
    sequence: u32,
    patch: AdjustmentPatch,
    created_at_millis: i64,
}

#[derive(Deserialize)]
struct StoredProcessingRecipeRevision {
    revision_id: ProcessingRecipeRevisionId,
    recipe_id: ProcessingRecipeId,
    sequence: u32,
    patch: AdjustmentPatch,
    created_at_millis: i64,
}

impl<'de> Deserialize<'de> for ProcessingRecipeRevision {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let stored = StoredProcessingRecipeRevision::deserialize(deserializer)?;
        Self::new(
            stored.revision_id,
            stored.recipe_id,
            stored.sequence,
            stored.patch,
            stored.created_at_millis,
        )
        .map_err(D::Error::custom)
    }
}

impl ProcessingRecipeRevision {
    /// Creates one validated immutable recipe revision.
    ///
    /// # Errors
    ///
    /// Revision sequence numbers start at one and timestamps must be
    /// non-negative. The embedded patch must also satisfy its full contract.
    pub fn new(
        revision_id: ProcessingRecipeRevisionId,
        recipe_id: ProcessingRecipeId,
        sequence: u32,
        patch: AdjustmentPatch,
        created_at_millis: i64,
    ) -> Result<Self, ProcessingRecipeError> {
        if sequence == 0 {
            return Err(ProcessingRecipeError::InvalidRevisionSequence);
        }
        if created_at_millis < 0 {
            return Err(ProcessingRecipeError::InvalidTimestamp);
        }
        patch.validate()?;
        Ok(Self {
            revision_id,
            recipe_id,
            sequence,
            patch,
            created_at_millis,
        })
    }

    #[must_use]
    pub const fn revision_id(&self) -> ProcessingRecipeRevisionId {
        self.revision_id
    }

    #[must_use]
    pub const fn recipe_id(&self) -> ProcessingRecipeId {
        self.recipe_id
    }

    #[must_use]
    pub const fn sequence(&self) -> u32 {
        self.sequence
    }

    #[must_use]
    pub const fn patch(&self) -> &AdjustmentPatch {
        &self.patch
    }

    #[must_use]
    pub const fn created_at_millis(&self) -> i64 {
        self.created_at_millis
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessingRecipeError {
    EmptyComponents,
    DuplicateComponent(ProcessingComponent),
    InvalidAdjustment(AdjustmentGraphError),
    InvalidEffectChain,
    InvalidRevisionSequence,
    InvalidTimestamp,
}

impl From<crate::EffectChainError> for ProcessingRecipeError {
    fn from(_: crate::EffectChainError) -> Self {
        Self::InvalidEffectChain
    }
}

impl std::fmt::Display for ProcessingRecipeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyComponents => {
                formatter.write_str("processing recipe must select a component")
            }
            Self::DuplicateComponent(component) => {
                write!(
                    formatter,
                    "processing recipe repeats component {component:?}"
                )
            }
            Self::InvalidAdjustment(error) => {
                write!(formatter, "processing recipe is invalid: {error}")
            }
            Self::InvalidEffectChain => {
                formatter.write_str("processing recipe effect chain is invalid")
            }
            Self::InvalidRevisionSequence => {
                formatter.write_str("processing recipe revision sequence must start at one")
            }
            Self::InvalidTimestamp => {
                formatter.write_str("processing recipe timestamp must be non-negative")
            }
        }
    }
}

impl std::error::Error for ProcessingRecipeError {}

#[cfg(test)]
mod tests;
