//! Authoritative construction and identity checks for the current development
//! Catalog schema.
//!
//! This module creates the current dated revision atomically and names the
//! immediately preceding compatible revision used by Catalog's focused
//! migration. Revisions use the canonical `YYYYMMDD.N` form.

use std::str::FromStr;

/// One dated Catalog contract revision: calendar date plus that day's
/// positive sequence number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CatalogSchemaRevision {
    date: u32,
    daily_sequence: u16,
}

impl CatalogSchemaRevision {
    /// Creates a canonical dated revision.
    #[must_use]
    pub(crate) const fn new(date: u32, daily_sequence: u16) -> Self {
        Self {
            date,
            daily_sequence,
        }
    }

    /// Eight-digit calendar identity in `YYYYMMDD` form.
    #[must_use]
    pub const fn date(self) -> u32 {
        self.date
    }

    /// Positive revision sequence within the date.
    #[must_use]
    pub const fn daily_sequence(self) -> u16 {
        self.daily_sequence
    }
}

impl std::fmt::Display for CatalogSchemaRevision {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:08}.{}", self.date, self.daily_sequence)
    }
}

/// A persisted schema revision that is not canonical `YYYYMMDD.N`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("schema revision must use canonical YYYYMMDD.N form")]
pub struct CatalogSchemaRevisionParseError;

impl FromStr for CatalogSchemaRevision {
    type Err = CatalogSchemaRevisionParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (date, sequence) = text
            .split_once('.')
            .ok_or(CatalogSchemaRevisionParseError)?;
        if date.len() != 8
            || !date.bytes().all(|byte| byte.is_ascii_digit())
            || sequence.is_empty()
            || !sequence.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(CatalogSchemaRevisionParseError);
        }
        let revision = Self::new(
            date.parse().map_err(|_| CatalogSchemaRevisionParseError)?,
            sequence
                .parse()
                .map_err(|_| CatalogSchemaRevisionParseError)?,
        );
        if !valid_calendar_date(revision.date)
            || revision.daily_sequence == 0
            || revision.to_string() != text
        {
            return Err(CatalogSchemaRevisionParseError);
        }
        Ok(revision)
    }
}

fn valid_calendar_date(date: u32) -> bool {
    let year = date / 10_000;
    let month = (date / 100) % 100;
    let day = date % 100;
    if year == 0 || !(1..=12).contains(&month) {
        return false;
    }
    let leap_year =
        year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days_in_month = match month {
        2 if leap_year => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    (1..=days_in_month).contains(&day)
}

pub(crate) const AUDIO_SEMANTIC_PREDECESSOR_SCHEMA_VERSION: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_813, 5);
pub(crate) const PREVIOUS_SCHEMA_VERSION: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_811, 16);
pub(crate) const CHANNEL_REPAIR_SCHEMA_VERSION: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_811, 15);
pub(crate) const DE_PLOSIVE_SCHEMA_VERSION: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_811, 14);
pub(crate) const SOURCE_EDIT_SCHEMA_VERSION: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_811, 13);
pub(crate) const PROCESSING_RECIPE_MANAGEMENT_SCHEMA_VERSION: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_811, 12);
pub(crate) const PROCESSING_RECIPES_SCHEMA_VERSION: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_811, 11);
pub(crate) const RESTORATIVE_EFFECTS_SCHEMA_VERSION: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_811, 10);
pub(crate) const EDITABLE_EFFECT_CHAIN_SCHEMA_VERSION: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_811, 9);
pub(crate) const FIXED_EFFECT_CHAIN_SCHEMA_VERSION: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_811, 8);
pub(crate) const LEGACY_SCHEMA_VERSION: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_811, 7);
pub(crate) const OLDER_COMPATIBLE_SCHEMA_VERSION: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_811, 6);
pub(crate) const OLDEST_COMPATIBLE_SCHEMA_VERSION: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_811, 5);
pub(crate) const ANCIENT_COMPATIBLE_SCHEMA_VERSION: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_811, 4);
pub(crate) const PRIMITIVE_COMPATIBLE_SCHEMA_VERSION: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_811, 3);
pub(crate) const EARLIEST_COMPATIBLE_SCHEMA_VERSION: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_811, 2);
pub(crate) const INITIAL_COMPATIBLE_SCHEMA_VERSION: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_811, 1);
pub(crate) const SPACE_CHARACTERS_SCHEMA_VERSION: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_812, 1);
pub(crate) const CREATIVE_VFX_SCHEMA_VERSION: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_812, 2);
pub(crate) const LISTENING_STATE_SCHEMA_VERSION: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_812, 3);
pub(crate) const DETERMINISTIC_VFX_SCHEMA_VERSION: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_812, 4);
pub(crate) const DRIVE_ROTARY_SCHEMA_VERSION: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_813, 1);
pub(crate) const CONVOLUTION_SPACE_SCHEMA_VERSION: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_813, 2);
pub(crate) const METADATA_CALIBRATION_SCHEMA_VERSION: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_813, 3);
pub(crate) const FREEZE_GRANULAR_SCHEMA_VERSION: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_813, 4);
pub(crate) const ORIGINAL_FIRST_SPECTRAL_SCHEMA_VERSION: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_815, 1);
pub(crate) const RENDERED_SPECTRAL_WORKING_COPY_SCHEMA_VERSION: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_815, 2);
pub(crate) const RENDERED_SPECTRAL_WORKING_COPY_EDIT_SCHEMA_VERSION: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_815, 3);
pub(crate) const RENDERED_SPECTRAL_WORKING_COPY_EXPORT_SCHEMA_VERSION: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_815, 4);
pub(crate) const SOUND_ASSEMBLY_PREDECESSOR_SCHEMA_VERSION: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_815, 5);
pub(crate) const SOUND_LIBRARY_PREDECESSOR_SCHEMA_VERSION: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_831, 1);
pub(crate) const SELECTION_TRANSCRIPT_PREDECESSOR: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_920, 1);
pub(crate) const SOURCE_DISCLOSURE_PREDECESSOR: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_920, 2);
pub(crate) const SCHEMA_VERSION: CatalogSchemaRevision = CatalogSchemaRevision::new(20_260_922, 1);

pub(crate) const SCHEMA_IDENTITY: &str = "echo-catalog-20260922.1-source-disclosures";

pub(crate) const SOUND_ASSEMBLY_MIGRATION_SQL: &str = r"
CREATE TABLE IF NOT EXISTS sound_assemblies (
    id                 TEXT PRIMARY KEY,
    created_at_millis  INTEGER NOT NULL CHECK (created_at_millis >= 0),
    updated_at_millis  INTEGER NOT NULL CHECK (updated_at_millis >= created_at_millis),
    archived_at_millis INTEGER CHECK (archived_at_millis >= updated_at_millis)
);

CREATE TABLE IF NOT EXISTS sound_assembly_revisions (
    id                    INTEGER PRIMARY KEY AUTOINCREMENT,
    assembly_id           TEXT NOT NULL REFERENCES sound_assemblies(id),
    revision_number       INTEGER NOT NULL CHECK (revision_number > 0),
    name                  TEXT NOT NULL CHECK (length(trim(name)) > 0),
    document_json         TEXT NOT NULL CHECK (json_valid(document_json)),
    duration_millis       INTEGER NOT NULL CHECK (duration_millis > 0),
    track_count           INTEGER NOT NULL CHECK (track_count BETWEEN 1 AND 8),
    clip_count            INTEGER NOT NULL CHECK (clip_count BETWEEN 1 AND 256),
    created_at_millis     INTEGER NOT NULL CHECK (created_at_millis >= 0),
    UNIQUE (assembly_id, revision_number)
);

CREATE INDEX IF NOT EXISTS sound_assembly_revisions_latest
    ON sound_assembly_revisions (assembly_id, revision_number DESC);

CREATE TABLE IF NOT EXISTS sound_assembly_clip_sources (
    assembly_revision_id  INTEGER NOT NULL REFERENCES sound_assembly_revisions(id),
    track_id              TEXT NOT NULL,
    clip_id               TEXT NOT NULL,
    asset_id              TEXT NOT NULL REFERENCES assets(id),
    adjustment_revision_id INTEGER REFERENCES asset_adjustment_revisions(id),
    source_start_millis   INTEGER NOT NULL CHECK (source_start_millis >= 0),
    source_end_millis     INTEGER NOT NULL CHECK (source_end_millis > source_start_millis),
    timeline_start_millis INTEGER NOT NULL CHECK (timeline_start_millis >= 0),
    PRIMARY KEY (assembly_revision_id, clip_id)
);

CREATE INDEX IF NOT EXISTS sound_assembly_clip_sources_asset
    ON sound_assembly_clip_sources (asset_id, adjustment_revision_id);

CREATE TABLE IF NOT EXISTS sound_assembly_exports (
    id                    INTEGER PRIMARY KEY AUTOINCREMENT,
    assembly_id           TEXT NOT NULL REFERENCES sound_assemblies(id),
    assembly_revision_id  INTEGER NOT NULL REFERENCES sound_assembly_revisions(id),
    output_path           TEXT NOT NULL,
    format                TEXT NOT NULL CHECK (format = 'wav_pcm24'),
    sample_rate           INTEGER NOT NULL CHECK (sample_rate > 0),
    channel_count         INTEGER NOT NULL CHECK (channel_count IN (1, 2)),
    bit_depth             INTEGER NOT NULL CHECK (bit_depth = 24),
    frame_count           INTEGER NOT NULL CHECK (frame_count > 0),
    content_hash          TEXT NOT NULL,
    size_bytes            INTEGER NOT NULL CHECK (size_bytes > 0),
    integrated_lufs       REAL NOT NULL,
    true_peak_dbtp        REAL NOT NULL,
    provenance_json       TEXT NOT NULL CHECK (json_valid(provenance_json)),
    created_at_millis     INTEGER NOT NULL CHECK (created_at_millis >= 0)
);

CREATE INDEX IF NOT EXISTS sound_assembly_exports_created
    ON sound_assembly_exports (assembly_id, created_at_millis DESC, id DESC);
";

pub(crate) const ORIGINAL_FIRST_SPECTRAL_MIGRATION_SQL: &str = r#"
ALTER TABLE asset_adjustment_revisions
    ADD COLUMN spectral_repair_json TEXT NOT NULL DEFAULT
    '{"enabled":true,"regions":[]}';
"#;

pub(crate) const RENDERED_SPECTRAL_WORKING_COPY_MIGRATION_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS rendered_spectral_working_copies (
    id                            INTEGER PRIMARY KEY AUTOINCREMENT,
    asset_id                      TEXT NOT NULL REFERENCES assets(id),
    parent_adjustment_revision_id INTEGER REFERENCES asset_adjustment_revisions(id),
    parent_render_content_hash    TEXT NOT NULL,
    working_render_content_hash   TEXT NOT NULL,
    manifest_schema_version       INTEGER NOT NULL CHECK (manifest_schema_version > 0),
    tile_manifest_json            TEXT NOT NULL CHECK (json_valid(tile_manifest_json)),
    tool_version                  TEXT NOT NULL CHECK (length(trim(tool_version)) > 0),
    enabled                       INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    created_at_millis             INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS rendered_spectral_working_copies_asset_created
    ON rendered_spectral_working_copies (asset_id, created_at_millis DESC, id DESC);
"#;

pub(crate) const RENDERED_SPECTRAL_WORKING_COPY_EDIT_MIGRATION_SQL: &str = r#"
ALTER TABLE rendered_spectral_working_copies
    ADD COLUMN working_render_content_hash TEXT NOT NULL DEFAULT '';

UPDATE rendered_spectral_working_copies
   SET working_render_content_hash = parent_render_content_hash
 WHERE working_render_content_hash = '';

UPDATE rendered_spectral_working_copies
   SET tile_manifest_json = '{"schema":1,"operations":[]}'
 WHERE tile_manifest_json = '{"schema":1,"tiles":[]}';
"#;

pub(crate) const RENDERED_SPECTRAL_WORKING_COPY_EXPORT_MIGRATION_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS render_export_working_copy_provenance (
    render_export_id              INTEGER PRIMARY KEY REFERENCES render_exports(id),
    working_copy_id               INTEGER NOT NULL REFERENCES rendered_spectral_working_copies(id),
    original_content_hash         TEXT NOT NULL,
    parent_render_content_hash    TEXT NOT NULL,
    working_render_content_hash   TEXT NOT NULL,
    tile_manifest_json            TEXT NOT NULL CHECK (json_valid(tile_manifest_json)),
    tool_version                  TEXT NOT NULL CHECK (length(trim(tool_version)) > 0)
);
"#;

pub(crate) const AUDIO_SEMANTIC_MIGRATION_SQL: &str = r"
CREATE TABLE IF NOT EXISTS audio_semantic_segments (
    asset_id           TEXT NOT NULL REFERENCES assets(id),
    start_millis       INTEGER NOT NULL CHECK (start_millis >= 0),
    end_millis         INTEGER NOT NULL CHECK (end_millis > start_millis),
    source_revision    TEXT NOT NULL,
    embedding_space    TEXT NOT NULL,
    vector             BLOB NOT NULL CHECK (length(vector) = 2048),
    runtime_json       TEXT NOT NULL CHECK (json_valid(runtime_json)),
    updated_at_millis  INTEGER NOT NULL CHECK (updated_at_millis >= 0),
    PRIMARY KEY (asset_id, start_millis, end_millis)
);
CREATE INDEX IF NOT EXISTS audio_semantic_segments_space
    ON audio_semantic_segments (embedding_space, asset_id, start_millis);
";
pub(crate) const METADATA_CALIBRATION_MIGRATION_SQL: &str = r"
CREATE TABLE IF NOT EXISTS metadata_calibration_revisions (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    asset_id           TEXT NOT NULL REFERENCES assets(id),
    sound_caption      TEXT,
    summary            TEXT,
    event_type         TEXT,
    mood               TEXT,
    keywords_json      TEXT CHECK (keywords_json IS NULL OR json_valid(keywords_json)),
    transcript_text    TEXT,
    language           TEXT,
    created_at_millis  INTEGER NOT NULL CHECK (created_at_millis >= 0)
);

CREATE INDEX IF NOT EXISTS metadata_calibration_revisions_asset
    ON metadata_calibration_revisions (asset_id, id DESC);
";
pub(crate) const CONVOLUTION_SPACE_MIGRATION_SQL: &str = r#"
ALTER TABLE asset_adjustment_revisions
    ADD COLUMN space_json TEXT NOT NULL DEFAULT
    '{"mode":"algorithmic","impulse_response":null,"convolution_mix_percent":35,"convolution_wet_gain_centibels":0}';

CREATE TABLE impulse_response_sources (
    source_hash       TEXT PRIMARY KEY,
    size_bytes        INTEGER NOT NULL CHECK (size_bytes > 0),
    created_at_millis INTEGER NOT NULL CHECK (created_at_millis >= 0)
);

CREATE TABLE impulse_response_preparations (
    prepared_hash       TEXT PRIMARY KEY,
    source_hash         TEXT NOT NULL REFERENCES impulse_response_sources(source_hash),
    size_bytes          INTEGER NOT NULL CHECK (size_bytes > 0),
    preparation_version INTEGER NOT NULL CHECK (preparation_version > 0),
    source_sample_rate  INTEGER NOT NULL CHECK (source_sample_rate > 0),
    channel_count       INTEGER NOT NULL CHECK (channel_count IN (1, 2)),
    source_frame_count  INTEGER NOT NULL CHECK (source_frame_count > 0),
    prepared_frame_count INTEGER NOT NULL CHECK (prepared_frame_count > 0),
    avcodec_version     INTEGER NOT NULL CHECK (avcodec_version > 0),
    swresample_version  INTEGER NOT NULL CHECK (swresample_version > 0),
    created_at_millis   INTEGER NOT NULL CHECK (created_at_millis >= 0)
);

CREATE TABLE impulse_response_imports (
    import_id          TEXT PRIMARY KEY,
    source_hash        TEXT NOT NULL REFERENCES impulse_response_sources(source_hash),
    prepared_hash      TEXT NOT NULL REFERENCES impulse_response_preparations(prepared_hash),
    imported_at_millis INTEGER NOT NULL CHECK (imported_at_millis >= 0),
    original_path      TEXT NOT NULL,
    display_name       TEXT NOT NULL CHECK (length(trim(display_name)) > 0),
    creator            TEXT,
    source_url         TEXT,
    attribution        TEXT,
    rights_kind        TEXT NOT NULL CHECK (
                       rights_kind IN ('spdx', 'user_owned_no_redistribution')),
    spdx_expression    TEXT,
    license_url        TEXT,
    CHECK ((rights_kind = 'spdx' AND length(trim(spdx_expression)) > 0) OR
           (rights_kind = 'user_owned_no_redistribution' AND spdx_expression IS NULL
            AND license_url IS NULL))
);

CREATE INDEX impulse_response_imports_newest
    ON impulse_response_imports (imported_at_millis DESC, import_id DESC);
"#;
pub(crate) const TRUE_STEREO_IR_MIGRATION_SQL: &str = r"
ALTER TABLE impulse_response_imports RENAME TO impulse_response_imports_v4;
ALTER TABLE impulse_response_preparations RENAME TO impulse_response_preparations_v4;

CREATE TABLE impulse_response_preparations (
    prepared_hash       TEXT PRIMARY KEY,
    source_hash         TEXT NOT NULL REFERENCES impulse_response_sources(source_hash),
    size_bytes          INTEGER NOT NULL CHECK (size_bytes > 0),
    preparation_version INTEGER NOT NULL CHECK (preparation_version > 0),
    source_sample_rate  INTEGER NOT NULL CHECK (source_sample_rate > 0),
    channel_count       INTEGER NOT NULL CHECK (channel_count IN (1, 2, 4)),
    layout_kind         TEXT NOT NULL CHECK (
                        layout_kind IN ('mono', 'stereo_parallel',
                                        'true_stereo_ll_lr_rl_rr')),
    source_frame_count  INTEGER NOT NULL CHECK (source_frame_count > 0),
    prepared_frame_count INTEGER NOT NULL CHECK (prepared_frame_count > 0),
    avcodec_version     INTEGER NOT NULL CHECK (avcodec_version > 0),
    swresample_version  INTEGER NOT NULL CHECK (swresample_version > 0),
    created_at_millis   INTEGER NOT NULL CHECK (created_at_millis >= 0),
    CHECK ((preparation_version = 1 AND channel_count = 1 AND layout_kind = 'mono') OR
           (preparation_version = 1 AND channel_count = 2 AND
            layout_kind = 'stereo_parallel') OR
           (preparation_version = 2 AND channel_count = 4 AND
            layout_kind = 'true_stereo_ll_lr_rl_rr'))
);

INSERT INTO impulse_response_preparations
    (prepared_hash, source_hash, size_bytes, preparation_version,
     source_sample_rate, channel_count, layout_kind, source_frame_count,
     prepared_frame_count, avcodec_version, swresample_version, created_at_millis)
SELECT prepared_hash, source_hash, size_bytes, preparation_version,
       source_sample_rate, channel_count,
       CASE channel_count WHEN 1 THEN 'mono' ELSE 'stereo_parallel' END,
       source_frame_count, prepared_frame_count, avcodec_version,
       swresample_version, created_at_millis
FROM impulse_response_preparations_v4;

CREATE TABLE impulse_response_imports (
    import_id          TEXT PRIMARY KEY,
    source_hash        TEXT NOT NULL REFERENCES impulse_response_sources(source_hash),
    prepared_hash      TEXT NOT NULL REFERENCES impulse_response_preparations(prepared_hash),
    imported_at_millis INTEGER NOT NULL CHECK (imported_at_millis >= 0),
    original_path      TEXT NOT NULL,
    display_name       TEXT NOT NULL CHECK (length(trim(display_name)) > 0),
    creator            TEXT,
    source_url         TEXT,
    attribution        TEXT,
    rights_kind        TEXT NOT NULL CHECK (
                       rights_kind IN ('spdx', 'user_owned_no_redistribution')),
    spdx_expression    TEXT,
    license_url        TEXT,
    CHECK ((rights_kind = 'spdx' AND length(trim(spdx_expression)) > 0) OR
           (rights_kind = 'user_owned_no_redistribution' AND spdx_expression IS NULL
            AND license_url IS NULL))
);

INSERT INTO impulse_response_imports
    (import_id, source_hash, prepared_hash, imported_at_millis, original_path,
     display_name, creator, source_url, attribution, rights_kind,
     spdx_expression, license_url)
SELECT import_id, source_hash, prepared_hash, imported_at_millis, original_path,
       display_name, creator, source_url, attribution, rights_kind,
       spdx_expression, license_url
FROM impulse_response_imports_v4;

DROP TABLE impulse_response_imports_v4;
DROP TABLE impulse_response_preparations_v4;
CREATE INDEX impulse_response_imports_newest
    ON impulse_response_imports (imported_at_millis DESC, import_id DESC);
";
pub(crate) const LISTENING_STATE_MIGRATION_SQL: &str = r"
ALTER TABLE asset_user_state
    ADD COLUMN last_listened_at_millis INTEGER NOT NULL DEFAULT 0
    CHECK (last_listened_at_millis >= 0);
ALTER TABLE asset_user_state
    ADD COLUMN resume_position_millis INTEGER NOT NULL DEFAULT 0
    CHECK (resume_position_millis >= 0);
";

pub(crate) const CREATIVE_VFX_MIGRATION_SQL: &str = r#"
ALTER TABLE asset_adjustment_revisions
    ADD COLUMN creative_vfx_json TEXT NOT NULL DEFAULT
    '{"scene":{"character":"telephone","enabled":false,"mix_percent":100,"intensity_percent":50},"delay":{"character":"slapback","enabled":false,"slapback":{"delay_millis":90,"mix_percent":22,"high_cut_hertz":7000},"echo":{"delay_millis":375,"feedback_percent":36,"mix_percent":28,"high_cut_hertz":6500,"stereo_crossfeed_percent":70}},"modulation":{"character":"chorus","enabled":false,"chorus":{"mix_percent":35,"rate_millihertz":800,"minimum_delay_microseconds":8000,"sweep_microseconds":10000,"stereo_phase_degrees":90},"flanger":{"mix_percent":50,"rate_millihertz":250,"minimum_delay_microseconds":200,"sweep_microseconds":3500,"feedback_percent":35,"stereo_phase_degrees":180},"phaser":{"mix_percent":50,"rate_millihertz":350,"sweep_low_hertz":300,"sweep_high_hertz":2500,"feedback_percent":25,"stereo_phase_degrees":90},"tremolo":{"rate_millihertz":4000,"depth_percent":60,"stereo_phase_degrees":0}},"transform":{"character":"robot","enabled":false,"mix_percent":100,"amount_percent":50}}';
"#;

pub(crate) const CHANNEL_REPAIR_MIGRATION_SQL: &str = r#"
ALTER TABLE asset_adjustment_revisions
    ADD COLUMN channel_repair_json TEXT NOT NULL DEFAULT
    '{"enabled":false,"invert_left":false,"invert_right":false,"swap_channels":false,"mono_fold_down":false,"balance_percent":0}';
"#;

pub(crate) const SOURCE_EDIT_MIGRATION_SQL: &str = r"
ALTER TABLE asset_adjustment_revisions
    ADD COLUMN edit_timeline_json TEXT NOT NULL DEFAULT '';
ALTER TABLE asset_adjustment_revisions
    ADD COLUMN effect_masks_json TEXT NOT NULL DEFAULT '[]';
";

pub(crate) const PROCESSING_RECIPE_MANAGEMENT_MIGRATION_SQL: &str = r"
ALTER TABLE processing_recipes
    ADD COLUMN archived_at_millis INTEGER CHECK (archived_at_millis >= 0);

CREATE TABLE processing_recipe_application_reverts (
    id                          INTEGER PRIMARY KEY AUTOINCREMENT,
    application_batch_id        INTEGER NOT NULL UNIQUE
                                REFERENCES processing_recipe_application_batches(id),
    created_at_millis           INTEGER NOT NULL
);

CREATE TABLE processing_recipe_application_revert_targets (
    revert_id                   INTEGER NOT NULL
                                REFERENCES processing_recipe_application_reverts(id),
    asset_id                    TEXT NOT NULL,
    outcome                     TEXT NOT NULL CHECK (
                                outcome IN ('restored', 'unchanged', 'conflict', 'failed')),
    encountered_adjustment_revision_id INTEGER REFERENCES asset_adjustment_revisions(id),
    restored_adjustment_revision_id INTEGER REFERENCES asset_adjustment_revisions(id),
    failure_reason              TEXT,
    PRIMARY KEY (revert_id, asset_id),
    CHECK (
        (outcome = 'restored' AND encountered_adjustment_revision_id IS NOT NULL
         AND restored_adjustment_revision_id IS NOT NULL AND failure_reason IS NULL) OR
        (outcome = 'unchanged' AND restored_adjustment_revision_id IS NULL
         AND failure_reason IS NULL) OR
        (outcome = 'conflict' AND encountered_adjustment_revision_id IS NOT NULL
         AND restored_adjustment_revision_id IS NULL AND failure_reason IS NULL) OR
        (outcome = 'failed' AND restored_adjustment_revision_id IS NULL
         AND failure_reason IS NOT NULL)
    )
);

CREATE INDEX processing_recipe_application_revert_targets_asset
    ON processing_recipe_application_revert_targets (asset_id, revert_id DESC);
";

pub(crate) const PROCESSING_RECIPES_MIGRATION_SQL: &str = r"
CREATE TABLE processing_recipes (
    id                 TEXT PRIMARY KEY,
    name               TEXT NOT NULL COLLATE NOCASE UNIQUE,
    created_at_millis  INTEGER NOT NULL,
    updated_at_millis  INTEGER NOT NULL
);

CREATE TABLE processing_recipe_revisions (
    id                 TEXT PRIMARY KEY,
    recipe_id          TEXT NOT NULL REFERENCES processing_recipes(id),
    revision_number    INTEGER NOT NULL CHECK (revision_number > 0),
    patch_json         TEXT NOT NULL,
    created_at_millis  INTEGER NOT NULL,
    UNIQUE (recipe_id, revision_number)
);

CREATE INDEX processing_recipe_revisions_current
    ON processing_recipe_revisions (recipe_id, revision_number DESC);

CREATE TABLE processing_recipe_application_batches (
    id                         INTEGER PRIMARY KEY AUTOINCREMENT,
    recipe_id                  TEXT NOT NULL REFERENCES processing_recipes(id),
    recipe_revision_id         TEXT NOT NULL REFERENCES processing_recipe_revisions(id),
    merge_mode                 TEXT NOT NULL CHECK (merge_mode IN ('merge', 'replace_processing')),
    created_at_millis          INTEGER NOT NULL
);

CREATE TABLE processing_recipe_application_targets (
    batch_id                    INTEGER NOT NULL REFERENCES processing_recipe_application_batches(id),
    asset_id                   TEXT NOT NULL,
    outcome                    TEXT NOT NULL CHECK (outcome IN ('updated', 'unchanged', 'failed')),
    previous_adjustment_revision_id INTEGER REFERENCES asset_adjustment_revisions(id),
    resulting_adjustment_revision_id INTEGER REFERENCES asset_adjustment_revisions(id),
    failure_reason             TEXT,
    PRIMARY KEY (batch_id, asset_id),
    CHECK (
        (outcome = 'failed' AND resulting_adjustment_revision_id IS NULL AND failure_reason IS NOT NULL) OR
        (outcome IN ('updated', 'unchanged') AND resulting_adjustment_revision_id IS NOT NULL AND failure_reason IS NULL)
    )
);

CREATE INDEX processing_recipe_application_targets_asset
    ON processing_recipe_application_targets (asset_id, batch_id DESC);
";

pub(crate) const DE_HUM_MIGRATION_SQL: &str = r#"
ALTER TABLE asset_adjustment_revisions
    ADD COLUMN de_hum_json TEXT NOT NULL DEFAULT
    '{"enabled":false,"fundamental_hertz":50,"harmonic_count":4,"quality_tenths":300,"depth_centibels":2400}';
"#;

pub(crate) const DE_CLICK_MIGRATION_SQL: &str = r#"
ALTER TABLE asset_adjustment_revisions
    ADD COLUMN de_click_json TEXT NOT NULL DEFAULT
    '{"enabled":false,"sensitivity_percent":50,"maximum_click_microseconds":1000,"repair_percent":100}';
"#;

pub(crate) const EFFECT_CHAIN_MIGRATION_SQL: &str = r#"
ALTER TABLE asset_adjustment_revisions
    ADD COLUMN effect_chain_json TEXT NOT NULL DEFAULT
    '{"nodes":["restoration","equalizer","dynamics","space","master","de_hum","de_click"],"active_count":5}';
"#;

pub(crate) const DELIVERY_FORMATS_MIGRATION_SQL: &str = r"
DROP INDEX IF EXISTS render_exports_asset_created;
ALTER TABLE render_exports RENAME TO render_exports_legacy;

CREATE TABLE render_exports (
    id                     INTEGER PRIMARY KEY AUTOINCREMENT,
    asset_id               TEXT NOT NULL REFERENCES assets(id),
    adjustment_revision_id INTEGER REFERENCES asset_adjustment_revisions(id),
    output_path            TEXT NOT NULL,
    format                 TEXT NOT NULL CHECK (
                           format IN ('wav_pcm16', 'wav_pcm24', 'flac24')),
    sample_rate            INTEGER NOT NULL CHECK (sample_rate > 0),
    channel_count          INTEGER NOT NULL CHECK (channel_count > 0),
    bit_depth              INTEGER NOT NULL CHECK (
                           (format = 'wav_pcm16' AND bit_depth = 16) OR
                           (format IN ('wav_pcm24', 'flac24') AND bit_depth = 24)),
    frame_count            INTEGER NOT NULL CHECK (frame_count > 0),
    content_hash           TEXT NOT NULL,
    size_bytes             INTEGER NOT NULL CHECK (size_bytes > 0),
    integrated_lufs        REAL NOT NULL,
    true_peak_dbtp         REAL NOT NULL,
    created_at_millis      INTEGER NOT NULL
);

INSERT INTO render_exports (
    id, asset_id, adjustment_revision_id, output_path, format, sample_rate,
    channel_count, bit_depth, frame_count, content_hash, size_bytes,
    integrated_lufs, true_peak_dbtp, created_at_millis)
SELECT id, asset_id, adjustment_revision_id, output_path, format, sample_rate,
       channel_count, bit_depth, frame_count, content_hash, size_bytes,
       integrated_lufs, true_peak_dbtp, created_at_millis
FROM render_exports_legacy;

DROP TABLE render_exports_legacy;
CREATE INDEX render_exports_asset_created
    ON render_exports (asset_id, created_at_millis DESC, id DESC);
";

pub(crate) const RESTORATION_CHAIN_MIGRATION_SQL: &str = r#"
ALTER TABLE asset_adjustment_revisions
    ADD COLUMN restoration_json TEXT NOT NULL DEFAULT
    '{"de_plosive":{"enabled":false,"frequency_hertz":140,"sensitivity_percent":50,"reduction_centibels":1200,"release_millis":160},"noise_reduction":{"enabled":false,"reduction_centibels":900,"sensitivity_percent":50,"smoothing_millis":240},"de_esser":{"enabled":false,"frequency_hertz":6500,"threshold_centibels":-2400,"reduction_centibels":600}}';
"#;

pub(crate) const SEMANTIC_SEARCH_MIGRATION_SQL: &str = r"
CREATE TABLE IF NOT EXISTS semantic_documents (
    asset_id           TEXT PRIMARY KEY REFERENCES assets(id),
    source_revision    TEXT NOT NULL,
    document_text      TEXT NOT NULL,
    embedding_space    TEXT NOT NULL,
    dimensions         INTEGER NOT NULL CHECK (dimensions > 0),
    quantized_vector   BLOB NOT NULL,
    vector_norm_sq     INTEGER NOT NULL CHECK (vector_norm_sq > 0),
    runtime_json       TEXT NOT NULL,
    updated_at_millis  INTEGER NOT NULL,
    CHECK (length(quantized_vector) = dimensions)
);

CREATE INDEX IF NOT EXISTS semantic_documents_space
    ON semantic_documents (embedding_space, asset_id);

CREATE VIRTUAL TABLE IF NOT EXISTS semantic_document_fts USING fts5(
    asset_id UNINDEXED,
    text,
    tokenize = 'unicode61'
);
";

pub(crate) const LONG_AUDIO_MIGRATION_SQL: &str = r"
CREATE TABLE long_audio_segments (
    asset_id           TEXT NOT NULL REFERENCES assets(id),
    plan_version       INTEGER NOT NULL,
    segment_index      INTEGER NOT NULL CHECK (segment_index >= 0),
    start_millis       INTEGER NOT NULL CHECK (start_millis >= 0),
    end_millis         INTEGER NOT NULL CHECK (end_millis > start_millis),
    proxy_content_hash TEXT,
    proxy_size_bytes   INTEGER CHECK (proxy_size_bytes > 0),
    transcript_json    TEXT,
    alignment_json     TEXT,
    contextual_json    TEXT,
    updated_at_millis  INTEGER NOT NULL,
    PRIMARY KEY (asset_id, plan_version, segment_index),
    CHECK ((proxy_content_hash IS NULL) = (proxy_size_bytes IS NULL))
);

CREATE INDEX long_audio_segments_asset_range
    ON long_audio_segments (asset_id, plan_version, start_millis);

CREATE TABLE long_audio_outline_nodes (
    asset_id           TEXT NOT NULL REFERENCES assets(id),
    plan_version       INTEGER NOT NULL,
    level              INTEGER NOT NULL CHECK (level >= 0),
    node_index         INTEGER NOT NULL CHECK (node_index >= 0),
    start_millis       INTEGER NOT NULL CHECK (start_millis >= 0),
    end_millis         INTEGER NOT NULL CHECK (end_millis > start_millis),
    contextual_json    TEXT NOT NULL,
    updated_at_millis  INTEGER NOT NULL,
    PRIMARY KEY (asset_id, plan_version, level, node_index)
);

CREATE INDEX long_audio_outline_asset_level
    ON long_audio_outline_nodes (asset_id, plan_version, level, start_millis);
";

pub(crate) const ADJUSTMENT_EFFECTS_MIGRATION_SQL: &str = r#"
ALTER TABLE asset_adjustment_revisions
    ADD COLUMN reverb_json TEXT NOT NULL DEFAULT
    '{"character":"room","enabled":false,"mix_percent":18,"pre_delay_millis":20,"decay_millis":1800,"size_percent":55,"damping_percent":45,"low_cut_hertz":120,"high_cut_hertz":10000}';
"#;

pub(crate) const RENDER_EXPORTS_MIGRATION_SQL: &str = r"
CREATE TABLE render_exports (
    id                     INTEGER PRIMARY KEY AUTOINCREMENT,
    asset_id               TEXT NOT NULL REFERENCES assets(id),
    adjustment_revision_id INTEGER REFERENCES asset_adjustment_revisions(id),
    output_path            TEXT NOT NULL,
    format                 TEXT NOT NULL CHECK (format = 'wav_pcm24'),
    sample_rate            INTEGER NOT NULL CHECK (sample_rate > 0),
    channel_count          INTEGER NOT NULL CHECK (channel_count > 0),
    bit_depth              INTEGER NOT NULL CHECK (bit_depth = 24),
    frame_count            INTEGER NOT NULL CHECK (frame_count > 0),
    content_hash           TEXT NOT NULL,
    size_bytes             INTEGER NOT NULL CHECK (size_bytes > 0),
    integrated_lufs        REAL NOT NULL,
    true_peak_dbtp         REAL NOT NULL,
    created_at_millis      INTEGER NOT NULL
);

CREATE INDEX render_exports_asset_created
    ON render_exports (asset_id, created_at_millis DESC, id DESC);
";

pub(crate) const USER_ALBUMS_MIGRATION_SQL: &str = r"
CREATE TABLE user_albums (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    name               TEXT NOT NULL COLLATE NOCASE UNIQUE,
    cover_asset_id     TEXT REFERENCES assets(id),
    created_at_millis  INTEGER NOT NULL,
    updated_at_millis  INTEGER NOT NULL
);

CREATE TABLE user_album_members (
    album_id           INTEGER NOT NULL REFERENCES user_albums(id),
    asset_id           TEXT NOT NULL REFERENCES assets(id),
    added_at_millis    INTEGER NOT NULL,
    PRIMARY KEY (album_id, asset_id)
);

CREATE INDEX user_album_members_asset
    ON user_album_members (asset_id, album_id);
";

pub(crate) const SCHEMA_SQL: &str = "
CREATE TABLE IF NOT EXISTS catalog_meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS assets (
    id                 TEXT PRIMARY KEY,
    content_hash       TEXT NOT NULL UNIQUE,
    path               TEXT NOT NULL,
    path_status        TEXT NOT NULL DEFAULT 'present'
                       CHECK (path_status IN ('present', 'missing')),
    size_bytes         INTEGER NOT NULL,
    codec              TEXT,
    duration_millis    INTEGER,
    recorded_at_millis INTEGER,
    imported_at_millis INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS analysis_records (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    asset_id           TEXT NOT NULL REFERENCES assets(id),
    kind               TEXT NOT NULL,
    value              TEXT NOT NULL,
    model              TEXT NOT NULL,
    model_version      TEXT NOT NULL,
    confidence         REAL,
    recorded_at_millis INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS analysis_records_asset_kind
    ON analysis_records (asset_id, kind);

CREATE TABLE IF NOT EXISTS contextual_browse_facets (
    analysis_record_id  INTEGER NOT NULL REFERENCES analysis_records(id) ON DELETE CASCADE,
    asset_id            TEXT NOT NULL REFERENCES assets(id),
    facet_kind          TEXT NOT NULL CHECK (
                        facet_kind IN ('keyword', 'mood', 'place', 'event', 'person')),
    normalized_value    TEXT NOT NULL,
    display_value       TEXT NOT NULL,
    PRIMARY KEY (analysis_record_id, facet_kind, normalized_value)
);

CREATE INDEX IF NOT EXISTS contextual_browse_facets_lookup
    ON contextual_browse_facets (facet_kind, normalized_value, asset_id);

CREATE INDEX IF NOT EXISTS contextual_browse_facets_latest
    ON contextual_browse_facets (asset_id, analysis_record_id DESC);

CREATE TABLE IF NOT EXISTS metadata_calibration_revisions (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    asset_id           TEXT NOT NULL REFERENCES assets(id),
    sound_caption      TEXT,
    summary            TEXT,
    event_type         TEXT,
    mood               TEXT,
    keywords_json      TEXT CHECK (keywords_json IS NULL OR json_valid(keywords_json)),
    transcript_text    TEXT,
    language           TEXT,
    created_at_millis  INTEGER NOT NULL CHECK (created_at_millis >= 0)
);

CREATE INDEX IF NOT EXISTS metadata_calibration_revisions_asset
    ON metadata_calibration_revisions (asset_id, id DESC);

CREATE TABLE IF NOT EXISTS asset_levels (
    asset_id   TEXT PRIMARY KEY REFERENCES assets(id),
    max_level  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS derived_artifacts (
    asset_id           TEXT NOT NULL REFERENCES assets(id),
    kind               TEXT NOT NULL,
    schema_version     INTEGER NOT NULL,
    content_hash       TEXT NOT NULL,
    size_bytes         INTEGER NOT NULL,
    created_at_millis  INTEGER NOT NULL,
    PRIMARY KEY (asset_id, kind, schema_version)
);

CREATE TABLE IF NOT EXISTS scan_roots (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    root             TEXT NOT NULL UNIQUE,
    enabled          INTEGER NOT NULL DEFAULT 1,
    added_at_millis  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS jobs (
    id                TEXT PRIMARY KEY,
    kind              TEXT NOT NULL,
    payload           TEXT NOT NULL,
    state             TEXT NOT NULL
                      CHECK (state IN ('pending', 'running', 'done', 'failed', 'cancelled')),
    progress          INTEGER NOT NULL DEFAULT 0,
    attempts          INTEGER NOT NULL DEFAULT 0,
    created_at_millis INTEGER NOT NULL,
    updated_at_millis INTEGER NOT NULL,
    error             TEXT
);

CREATE INDEX IF NOT EXISTS jobs_state_kind ON jobs (state, kind);

CREATE TABLE IF NOT EXISTS scan_journal (
    path            TEXT PRIMARY KEY,
    size_bytes      INTEGER NOT NULL,
    mtime_millis    INTEGER NOT NULL,
    content_hash    TEXT NOT NULL
);

CREATE VIRTUAL TABLE IF NOT EXISTS transcript_fts USING fts5(
    asset_id UNINDEXED,
    text,
    tokenize = 'unicode61'
);

CREATE TABLE IF NOT EXISTS semantic_documents (
    asset_id           TEXT PRIMARY KEY REFERENCES assets(id),
    source_revision    TEXT NOT NULL,
    document_text      TEXT NOT NULL,
    embedding_space    TEXT NOT NULL,
    dimensions         INTEGER NOT NULL CHECK (dimensions > 0),
    quantized_vector   BLOB NOT NULL,
    vector_norm_sq     INTEGER NOT NULL CHECK (vector_norm_sq > 0),
    runtime_json       TEXT NOT NULL,
    updated_at_millis  INTEGER NOT NULL,
    CHECK (length(quantized_vector) = dimensions)
);

CREATE INDEX IF NOT EXISTS semantic_documents_space
    ON semantic_documents (embedding_space, asset_id);

CREATE VIRTUAL TABLE IF NOT EXISTS semantic_document_fts USING fts5(
    asset_id UNINDEXED,
    text,
    tokenize = 'unicode61'
);

CREATE TABLE IF NOT EXISTS asset_user_state (
    asset_id           TEXT PRIMARY KEY REFERENCES assets(id),
    liked              INTEGER NOT NULL DEFAULT 0 CHECK (liked IN (0, 1)),
    rating             INTEGER NOT NULL DEFAULT 0 CHECK (rating BETWEEN 0 AND 5),
    last_listened_at_millis INTEGER NOT NULL DEFAULT 0
                       CHECK (last_listened_at_millis >= 0),
    resume_position_millis INTEGER NOT NULL DEFAULT 0
                       CHECK (resume_position_millis >= 0),
    updated_at_millis  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS user_albums (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    name               TEXT NOT NULL COLLATE NOCASE UNIQUE,
    cover_asset_id     TEXT REFERENCES assets(id),
    created_at_millis  INTEGER NOT NULL,
    updated_at_millis  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS user_album_members (
    album_id           INTEGER NOT NULL REFERENCES user_albums(id),
    asset_id           TEXT NOT NULL REFERENCES assets(id),
    added_at_millis    INTEGER NOT NULL,
    PRIMARY KEY (album_id, asset_id)
);

CREATE INDEX IF NOT EXISTS user_album_members_asset
    ON user_album_members (asset_id, album_id);

CREATE TABLE IF NOT EXISTS long_audio_segments (
    asset_id           TEXT NOT NULL REFERENCES assets(id),
    plan_version       INTEGER NOT NULL,
    segment_index      INTEGER NOT NULL CHECK (segment_index >= 0),
    start_millis       INTEGER NOT NULL CHECK (start_millis >= 0),
    end_millis         INTEGER NOT NULL CHECK (end_millis > start_millis),
    proxy_content_hash TEXT,
    proxy_size_bytes   INTEGER CHECK (proxy_size_bytes > 0),
    transcript_json    TEXT,
    alignment_json     TEXT,
    contextual_json    TEXT,
    updated_at_millis  INTEGER NOT NULL,
    PRIMARY KEY (asset_id, plan_version, segment_index),
    CHECK ((proxy_content_hash IS NULL) = (proxy_size_bytes IS NULL))
);

CREATE INDEX IF NOT EXISTS long_audio_segments_asset_range
    ON long_audio_segments (asset_id, plan_version, start_millis);

CREATE TABLE IF NOT EXISTS long_audio_outline_nodes (
    asset_id           TEXT NOT NULL REFERENCES assets(id),
    plan_version       INTEGER NOT NULL,
    level              INTEGER NOT NULL CHECK (level >= 0),
    node_index         INTEGER NOT NULL CHECK (node_index >= 0),
    start_millis       INTEGER NOT NULL CHECK (start_millis >= 0),
    end_millis         INTEGER NOT NULL CHECK (end_millis > start_millis),
    contextual_json    TEXT NOT NULL,
    updated_at_millis  INTEGER NOT NULL,
    PRIMARY KEY (asset_id, plan_version, level, node_index)
);

CREATE INDEX IF NOT EXISTS long_audio_outline_asset_level
    ON long_audio_outline_nodes (asset_id, plan_version, level, start_millis);

CREATE TABLE IF NOT EXISTS asset_adjustment_revisions (
    id                    INTEGER PRIMARY KEY AUTOINCREMENT,
    asset_id              TEXT NOT NULL REFERENCES assets(id),
    trim_start_millis     INTEGER NOT NULL CHECK (trim_start_millis >= 0),
    trim_end_millis       INTEGER NOT NULL CHECK (trim_end_millis > trim_start_millis),
    fade_in_millis        INTEGER NOT NULL CHECK (fade_in_millis >= 0),
    fade_out_millis       INTEGER NOT NULL CHECK (fade_out_millis >= 0),
    fade_in_curve         INTEGER NOT NULL DEFAULT 0 CHECK (fade_in_curve IN (0, 1, 2)),
    fade_out_curve        INTEGER NOT NULL DEFAULT 0 CHECK (fade_out_curve IN (0, 1, 2)),
    gain_centibels        INTEGER NOT NULL CHECK (gain_centibels BETWEEN -2400 AND 1200),
    low_cut_hertz         INTEGER NOT NULL DEFAULT 0
                          CHECK (low_cut_hertz = 0 OR low_cut_hertz BETWEEN 20 AND 240),
    eq_low_gain_centibels INTEGER NOT NULL DEFAULT 0
                          CHECK (eq_low_gain_centibels BETWEEN -1200 AND 1200),
    eq_mid_gain_centibels INTEGER NOT NULL DEFAULT 0
                          CHECK (eq_mid_gain_centibels BETWEEN -1200 AND 1200),
    eq_high_gain_centibels INTEGER NOT NULL DEFAULT 0
                           CHECK (eq_high_gain_centibels BETWEEN -1200 AND 1200),
    parametric_equalizer_json TEXT NOT NULL DEFAULT
                           '{\"enabled\":true,\"bands\":[{\"enabled\":true,\"filter_kind\":\"low_shelf\",\"frequency_hertz\":120,\"q_hundredths\":71,\"gain_centibels\":0},{\"enabled\":false,\"filter_kind\":\"bell\",\"frequency_hertz\":250,\"q_hundredths\":100,\"gain_centibels\":0},{\"enabled\":true,\"filter_kind\":\"bell\",\"frequency_hertz\":1000,\"q_hundredths\":100,\"gain_centibels\":0},{\"enabled\":false,\"filter_kind\":\"bell\",\"frequency_hertz\":3000,\"q_hundredths\":100,\"gain_centibels\":0},{\"enabled\":false,\"filter_kind\":\"bell\",\"frequency_hertz\":5000,\"q_hundredths\":100,\"gain_centibels\":0},{\"enabled\":true,\"filter_kind\":\"high_shelf\",\"frequency_hertz\":8000,\"q_hundredths\":71,\"gain_centibels\":0}]}',
    compressor_enabled     INTEGER NOT NULL DEFAULT 0
                           CHECK (compressor_enabled IN (0, 1)),
    compressor_threshold_centibels INTEGER NOT NULL DEFAULT -1800
                           CHECK (compressor_threshold_centibels BETWEEN -6000 AND 0),
    compressor_ratio_tenths INTEGER NOT NULL DEFAULT 30
                           CHECK (compressor_ratio_tenths BETWEEN 10 AND 200),
    compressor_attack_millis INTEGER NOT NULL DEFAULT 10
                           CHECK (compressor_attack_millis BETWEEN 1 AND 200),
    compressor_release_millis INTEGER NOT NULL DEFAULT 120
                           CHECK (compressor_release_millis BETWEEN 20 AND 2000),
    compressor_makeup_centibels INTEGER NOT NULL DEFAULT 0
                           CHECK (compressor_makeup_centibels BETWEEN 0 AND 2400),
    reverb_json             TEXT NOT NULL DEFAULT
                           '{\"character\":\"room\",\"enabled\":false,\"mix_percent\":18,\"pre_delay_millis\":20,\"decay_millis\":1800,\"size_percent\":55,\"damping_percent\":45,\"low_cut_hertz\":120,\"high_cut_hertz\":10000}',
    space_json              TEXT NOT NULL DEFAULT
                           '{\"mode\":\"algorithmic\",\"impulse_response\":null,\"convolution_mix_percent\":35,\"convolution_wet_gain_centibels\":0}',
    creative_vfx_json       TEXT NOT NULL DEFAULT
                           '{\"scene\":{\"character\":\"telephone\",\"enabled\":false,\"mix_percent\":100,\"intensity_percent\":50},\"delay\":{\"character\":\"slapback\",\"enabled\":false,\"slapback\":{\"delay_millis\":90,\"mix_percent\":22,\"high_cut_hertz\":7000},\"echo\":{\"delay_millis\":375,\"feedback_percent\":36,\"mix_percent\":28,\"high_cut_hertz\":6500,\"stereo_crossfeed_percent\":70}},\"modulation\":{\"character\":\"chorus\",\"enabled\":false,\"chorus\":{\"mix_percent\":35,\"rate_millihertz\":800,\"minimum_delay_microseconds\":8000,\"sweep_microseconds\":10000,\"stereo_phase_degrees\":90},\"flanger\":{\"mix_percent\":50,\"rate_millihertz\":250,\"minimum_delay_microseconds\":200,\"sweep_microseconds\":3500,\"feedback_percent\":35,\"stereo_phase_degrees\":180},\"phaser\":{\"mix_percent\":50,\"rate_millihertz\":350,\"sweep_low_hertz\":300,\"sweep_high_hertz\":2500,\"feedback_percent\":25,\"stereo_phase_degrees\":90},\"tremolo\":{\"rate_millihertz\":4000,\"depth_percent\":60,\"stereo_phase_degrees\":0}},\"transform\":{\"character\":\"robot\",\"enabled\":false,\"mix_percent\":100,\"amount_percent\":50}}',
    spectral_repair_json     TEXT NOT NULL DEFAULT
                           '{\"enabled\":true,\"regions\":[]}',
    restoration_json        TEXT NOT NULL DEFAULT
                           '{\"enabled\":true,\"de_plosive\":{\"enabled\":false,\"frequency_hertz\":140,\"sensitivity_percent\":50,\"reduction_centibels\":1200,\"release_millis\":160},\"noise_reduction\":{\"enabled\":false,\"reduction_centibels\":900,\"sensitivity_percent\":50,\"smoothing_millis\":240},\"de_esser\":{\"enabled\":false,\"frequency_hertz\":6500,\"threshold_centibels\":-2400,\"reduction_centibels\":600}}',
    de_hum_json             TEXT NOT NULL DEFAULT
                           '{\"enabled\":false,\"fundamental_hertz\":50,\"harmonic_count\":4,\"quality_tenths\":300,\"depth_centibels\":2400}',
    de_click_json           TEXT NOT NULL DEFAULT
                           '{\"enabled\":false,\"sensitivity_percent\":50,\"maximum_click_microseconds\":1000,\"repair_percent\":100}',
    channel_repair_json     TEXT NOT NULL DEFAULT
                           '{\"enabled\":false,\"invert_left\":false,\"invert_right\":false,\"swap_channels\":false,\"mono_fold_down\":false,\"balance_percent\":0}',
    effect_chain_json       TEXT NOT NULL DEFAULT
                           '{\"nodes\":[\"restoration\",\"equalizer\",\"dynamics\",\"space\",\"master\",\"de_hum\",\"de_click\",\"channel_repair\",\"scene_vfx\",\"delay_vfx\",\"modulation_vfx\",\"transform_vfx\"],\"active_count\":5}',
    edit_timeline_json      TEXT NOT NULL DEFAULT '',
    effect_masks_json       TEXT NOT NULL DEFAULT '[]',
    limiter_enabled        INTEGER NOT NULL DEFAULT 0
                           CHECK (limiter_enabled IN (0, 1)),
    limiter_ceiling_centibels INTEGER NOT NULL DEFAULT -100
                           CHECK (limiter_ceiling_centibels BETWEEN -600 AND 0),
    limiter_release_millis INTEGER NOT NULL DEFAULT 100
                           CHECK (limiter_release_millis BETWEEN 20 AND 1000),
    created_at_millis     INTEGER NOT NULL,
    CHECK (fade_in_millis + fade_out_millis <= trim_end_millis - trim_start_millis)
);

CREATE INDEX IF NOT EXISTS asset_adjustment_revisions_latest
    ON asset_adjustment_revisions (asset_id, id DESC);

CREATE TABLE IF NOT EXISTS impulse_response_sources (
    source_hash       TEXT PRIMARY KEY,
    size_bytes        INTEGER NOT NULL CHECK (size_bytes > 0),
    created_at_millis INTEGER NOT NULL CHECK (created_at_millis >= 0)
);

CREATE TABLE IF NOT EXISTS impulse_response_preparations (
    prepared_hash       TEXT PRIMARY KEY,
    source_hash         TEXT NOT NULL REFERENCES impulse_response_sources(source_hash),
    size_bytes          INTEGER NOT NULL CHECK (size_bytes > 0),
    preparation_version INTEGER NOT NULL CHECK (preparation_version > 0),
    source_sample_rate  INTEGER NOT NULL CHECK (source_sample_rate > 0),
    channel_count       INTEGER NOT NULL CHECK (channel_count IN (1, 2, 4)),
    layout_kind         TEXT NOT NULL CHECK (
                        layout_kind IN ('mono', 'stereo_parallel',
                                        'true_stereo_ll_lr_rl_rr')),
    source_frame_count  INTEGER NOT NULL CHECK (source_frame_count > 0),
    prepared_frame_count INTEGER NOT NULL CHECK (prepared_frame_count > 0),
    avcodec_version     INTEGER NOT NULL CHECK (avcodec_version > 0),
    swresample_version  INTEGER NOT NULL CHECK (swresample_version > 0),
    created_at_millis   INTEGER NOT NULL CHECK (created_at_millis >= 0),
    CHECK ((preparation_version = 1 AND channel_count = 1 AND layout_kind = 'mono') OR
           (preparation_version = 1 AND channel_count = 2 AND
            layout_kind = 'stereo_parallel') OR
           (preparation_version = 2 AND channel_count = 4 AND
            layout_kind = 'true_stereo_ll_lr_rl_rr'))
);

CREATE TABLE IF NOT EXISTS impulse_response_imports (
    import_id          TEXT PRIMARY KEY,
    source_hash        TEXT NOT NULL REFERENCES impulse_response_sources(source_hash),
    prepared_hash      TEXT NOT NULL REFERENCES impulse_response_preparations(prepared_hash),
    imported_at_millis INTEGER NOT NULL CHECK (imported_at_millis >= 0),
    original_path      TEXT NOT NULL,
    display_name       TEXT NOT NULL CHECK (length(trim(display_name)) > 0),
    creator            TEXT,
    source_url         TEXT,
    attribution        TEXT,
    rights_kind        TEXT NOT NULL CHECK (
                       rights_kind IN ('spdx', 'user_owned_no_redistribution')),
    spdx_expression    TEXT,
    license_url        TEXT,
    CHECK ((rights_kind = 'spdx' AND length(trim(spdx_expression)) > 0) OR
           (rights_kind = 'user_owned_no_redistribution' AND spdx_expression IS NULL
            AND license_url IS NULL))
);

CREATE INDEX IF NOT EXISTS impulse_response_imports_newest
    ON impulse_response_imports (imported_at_millis DESC, import_id DESC);

CREATE TABLE IF NOT EXISTS processing_recipes (
    id                 TEXT PRIMARY KEY,
    name               TEXT NOT NULL COLLATE NOCASE UNIQUE,
    created_at_millis  INTEGER NOT NULL,
    updated_at_millis  INTEGER NOT NULL,
    archived_at_millis INTEGER CHECK (archived_at_millis >= 0)
);

CREATE TABLE IF NOT EXISTS processing_recipe_revisions (
    id                 TEXT PRIMARY KEY,
    recipe_id          TEXT NOT NULL REFERENCES processing_recipes(id),
    revision_number    INTEGER NOT NULL CHECK (revision_number > 0),
    patch_json         TEXT NOT NULL,
    created_at_millis  INTEGER NOT NULL,
    UNIQUE (recipe_id, revision_number)
);

CREATE INDEX IF NOT EXISTS processing_recipe_revisions_current
    ON processing_recipe_revisions (recipe_id, revision_number DESC);

CREATE TABLE IF NOT EXISTS processing_recipe_application_batches (
    id                         INTEGER PRIMARY KEY AUTOINCREMENT,
    recipe_id                  TEXT NOT NULL REFERENCES processing_recipes(id),
    recipe_revision_id         TEXT NOT NULL REFERENCES processing_recipe_revisions(id),
    merge_mode                 TEXT NOT NULL CHECK (merge_mode IN ('merge', 'replace_processing')),
    created_at_millis          INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS processing_recipe_application_targets (
    batch_id                    INTEGER NOT NULL REFERENCES processing_recipe_application_batches(id),
    asset_id                   TEXT NOT NULL,
    outcome                    TEXT NOT NULL CHECK (outcome IN ('updated', 'unchanged', 'failed')),
    previous_adjustment_revision_id INTEGER REFERENCES asset_adjustment_revisions(id),
    resulting_adjustment_revision_id INTEGER REFERENCES asset_adjustment_revisions(id),
    failure_reason             TEXT,
    PRIMARY KEY (batch_id, asset_id),
    CHECK (
        (outcome = 'failed' AND resulting_adjustment_revision_id IS NULL AND failure_reason IS NOT NULL) OR
        (outcome IN ('updated', 'unchanged') AND resulting_adjustment_revision_id IS NOT NULL AND failure_reason IS NULL)
    )
);

CREATE INDEX IF NOT EXISTS processing_recipe_application_targets_asset
    ON processing_recipe_application_targets (asset_id, batch_id DESC);

CREATE TABLE IF NOT EXISTS processing_recipe_application_reverts (
    id                          INTEGER PRIMARY KEY AUTOINCREMENT,
    application_batch_id        INTEGER NOT NULL UNIQUE
                                REFERENCES processing_recipe_application_batches(id),
    created_at_millis           INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS processing_recipe_application_revert_targets (
    revert_id                   INTEGER NOT NULL
                                REFERENCES processing_recipe_application_reverts(id),
    asset_id                    TEXT NOT NULL,
    outcome                     TEXT NOT NULL CHECK (
                                outcome IN ('restored', 'unchanged', 'conflict', 'failed')),
    encountered_adjustment_revision_id INTEGER REFERENCES asset_adjustment_revisions(id),
    restored_adjustment_revision_id INTEGER REFERENCES asset_adjustment_revisions(id),
    failure_reason              TEXT,
    PRIMARY KEY (revert_id, asset_id),
    CHECK (
        (outcome = 'restored' AND encountered_adjustment_revision_id IS NOT NULL
         AND restored_adjustment_revision_id IS NOT NULL AND failure_reason IS NULL) OR
        (outcome = 'unchanged' AND restored_adjustment_revision_id IS NULL
         AND failure_reason IS NULL) OR
        (outcome = 'conflict' AND encountered_adjustment_revision_id IS NOT NULL
         AND restored_adjustment_revision_id IS NULL AND failure_reason IS NULL) OR
        (outcome = 'failed' AND restored_adjustment_revision_id IS NULL
         AND failure_reason IS NOT NULL)
    )
);

CREATE INDEX IF NOT EXISTS processing_recipe_application_revert_targets_asset
    ON processing_recipe_application_revert_targets (asset_id, revert_id DESC);

CREATE TABLE IF NOT EXISTS render_exports (
    id                     INTEGER PRIMARY KEY AUTOINCREMENT,
    asset_id               TEXT NOT NULL REFERENCES assets(id),
    adjustment_revision_id INTEGER REFERENCES asset_adjustment_revisions(id),
    output_path            TEXT NOT NULL,
    format                 TEXT NOT NULL CHECK (
                           format IN ('wav_pcm16', 'wav_pcm24', 'flac24')),
    sample_rate            INTEGER NOT NULL CHECK (sample_rate > 0),
    channel_count          INTEGER NOT NULL CHECK (channel_count > 0),
    bit_depth              INTEGER NOT NULL CHECK (
                           (format = 'wav_pcm16' AND bit_depth = 16) OR
                           (format IN ('wav_pcm24', 'flac24') AND bit_depth = 24)),
    frame_count            INTEGER NOT NULL CHECK (frame_count > 0),
    content_hash           TEXT NOT NULL,
    size_bytes             INTEGER NOT NULL CHECK (size_bytes > 0),
    integrated_lufs        REAL NOT NULL,
    true_peak_dbtp         REAL NOT NULL,
    created_at_millis      INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS render_exports_asset_created
    ON render_exports (asset_id, created_at_millis DESC, id DESC);

CREATE TABLE IF NOT EXISTS rendered_spectral_working_copies (
    id                            INTEGER PRIMARY KEY AUTOINCREMENT,
    asset_id                      TEXT NOT NULL REFERENCES assets(id),
    parent_adjustment_revision_id INTEGER REFERENCES asset_adjustment_revisions(id),
    parent_render_content_hash    TEXT NOT NULL,
    working_render_content_hash   TEXT NOT NULL,
    manifest_schema_version       INTEGER NOT NULL CHECK (manifest_schema_version > 0),
    tile_manifest_json            TEXT NOT NULL CHECK (json_valid(tile_manifest_json)),
    tool_version                  TEXT NOT NULL CHECK (length(trim(tool_version)) > 0),
    enabled                       INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    created_at_millis             INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS rendered_spectral_working_copies_asset_created
    ON rendered_spectral_working_copies (asset_id, created_at_millis DESC, id DESC);

CREATE TABLE IF NOT EXISTS render_export_working_copy_provenance (
    render_export_id              INTEGER PRIMARY KEY REFERENCES render_exports(id),
    working_copy_id               INTEGER NOT NULL REFERENCES rendered_spectral_working_copies(id),
    original_content_hash         TEXT NOT NULL,
    parent_render_content_hash    TEXT NOT NULL,
    working_render_content_hash   TEXT NOT NULL,
    tile_manifest_json            TEXT NOT NULL CHECK (json_valid(tile_manifest_json)),
    tool_version                  TEXT NOT NULL CHECK (length(trim(tool_version)) > 0)
);

CREATE TABLE IF NOT EXISTS asset_source_metadata (
    asset_id           TEXT PRIMARY KEY REFERENCES assets(id),
    container_format   TEXT NOT NULL,
    sample_rate        INTEGER NOT NULL,
    channel_count      INTEGER NOT NULL,
    entries_json       TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS inference_runs (
    local_job_id       TEXT PRIMARY KEY REFERENCES jobs(id) ON DELETE CASCADE,
    asset_id           TEXT NOT NULL REFERENCES assets(id),
    intent             TEXT NOT NULL,
    runtime_job_id     TEXT,
    contract_version   TEXT NOT NULL,
    state              TEXT NOT NULL CHECK (
                       state IN ('submitting', 'succeeded', 'failed', 'cancelled', 'expired')),
    http_status        INTEGER,
    error_code         TEXT,
    snapshot_json      TEXT,
    updated_at_millis  INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS inference_runs_asset_intent
    ON inference_runs (asset_id, intent, updated_at_millis DESC);
";

#[cfg(test)]
mod tests;
