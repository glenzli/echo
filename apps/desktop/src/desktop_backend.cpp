#include "desktop_backend.hpp"

#include <QDateTime>
#include <QDebug>
#include <QDir>
#include <QFileInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QString>

#include <array>
#include <utility>

#include "creative_vfx_projection.hpp"
#include "parametric_equalizer_projection.hpp"
#include "playback_adjustment_projection.hpp"

namespace {

QVariantMap impulseResponseForQml(const echo::desktop::ImpulseResponseWire& wire) {
    QVariantMap value;
    value.insert(
        QStringLiteral("importId"),
        QString::fromUtf8(wire.import_id.data(), wire.import_id.size())
    );
    value.insert(
        QStringLiteral("sourceHash"),
        QString::fromUtf8(wire.source_hash.data(), wire.source_hash.size())
    );
    value.insert(
        QStringLiteral("preparedHash"),
        QString::fromUtf8(wire.prepared_hash.data(), wire.prepared_hash.size())
    );
    value.insert(
        QStringLiteral("preparedPath"),
        QString::fromUtf8(wire.prepared_path.data(), wire.prepared_path.size())
    );
    value.insert(
        QStringLiteral("displayName"),
        QString::fromUtf8(wire.display_name.data(), wire.display_name.size())
    );
    value.insert(
        QStringLiteral("creator"),
        QString::fromUtf8(wire.creator.data(), wire.creator.size())
    );
    value.insert(
        QStringLiteral("sourceUrl"),
        QString::fromUtf8(wire.source_url.data(), wire.source_url.size())
    );
    value.insert(
        QStringLiteral("attribution"),
        QString::fromUtf8(wire.attribution.data(), wire.attribution.size())
    );
    value.insert(
        QStringLiteral("rightsKind"),
        QString::fromUtf8(wire.rights_kind.data(), wire.rights_kind.size())
    );
    value.insert(
        QStringLiteral("spdxExpression"),
        QString::fromUtf8(wire.spdx_expression.data(), wire.spdx_expression.size())
    );
    value.insert(
        QStringLiteral("licenseUrl"),
        QString::fromUtf8(wire.license_url.data(), wire.license_url.size())
    );
    value.insert(
        QStringLiteral("importedAtMillis"),
        static_cast<qlonglong>(wire.imported_at_millis)
    );
    value.insert(QStringLiteral("sourceSampleRate"), static_cast<int>(wire.source_sample_rate));
    value.insert(QStringLiteral("channelCount"), static_cast<int>(wire.channel_count));
    value.insert(
        QStringLiteral("layoutKind"),
        QString::fromUtf8(wire.layout_kind.data(), wire.layout_kind.size())
    );
    value.insert(
        QStringLiteral("preparedFrameCount"),
        static_cast<qlonglong>(wire.prepared_frame_count)
    );
    return value;
}

QVariantMap creativeVfxForQml(const rust::String& encoded) {
    const auto adjustment = CreativeVfxProjection::fromJson(
        QByteArray(encoded.data(), static_cast<qsizetype>(encoded.size()))
    );
    return adjustment.has_value() ? CreativeVfxProjection::toQml(*adjustment) : QVariantMap{};
}

bool appendEqualizerBands(
    const QVariantList& values,
    rust::Vec<echo::desktop::EqualizerBandWire>& destination
) {
    const auto adjustment = ParametricEqualizerProjection::fromQml(values);
    if (!adjustment.has_value()) {
        return false;
    }
    for (const auto& band : adjustment->bands) {
        echo::desktop::EqualizerBandWire wire;
        wire.enabled = band.enabled;
        wire.filter_kind = static_cast<std::uint8_t>(band.filter_kind);
        wire.frequency_hertz = band.frequency_hertz;
        wire.q_hundredths = band.q_hundredths;
        wire.gain_centibels = band.gain_centibels;
        destination.push_back(wire);
    }
    return true;
}

QVariantList equalizerBandsForQml(const rust::Vec<echo::desktop::EqualizerBandWire>& bands) {
    QVariantList result;
    for (const auto& band : bands) {
        QVariantMap value;
        value.insert(QStringLiteral("enabled"), band.enabled);
        value.insert(QStringLiteral("filterKind"), static_cast<int>(band.filter_kind));
        value.insert(QStringLiteral("frequencyHertz"), static_cast<int>(band.frequency_hertz));
        value.insert(QStringLiteral("qHundredths"), static_cast<int>(band.q_hundredths));
        value.insert(QStringLiteral("gainCentibels"), static_cast<int>(band.gain_centibels));
        result.append(value);
    }
    return result;
}

bool appendEffectChain(const QVariantList& values, rust::Vec<std::uint8_t>& destination) {
    if (values.isEmpty() || values.size() > static_cast<qsizetype>(echo::audio::kEffectNodeCount)
        || values.back().toInt() != 4) {
        return false;
    }
    std::array<bool, echo::audio::kEffectNodeCount> seen{};
    for (const QVariant& item : values) {
        const int value = item.toInt();
        if (value < 0 || value >= static_cast<int>(seen.size())
            || seen[static_cast<std::size_t>(value)]) {
            return false;
        }
        seen[static_cast<std::size_t>(value)] = true;
        destination.push_back(static_cast<std::uint8_t>(value));
    }
    return true;
}

QVariantList effectChainForQml(const rust::Vec<std::uint8_t>& chain) {
    QVariantList result;
    for (const std::uint8_t node : chain) {
        result.append(static_cast<int>(node));
    }
    return result;
}

bool appendEditSegments(
    const QVariantList& values,
    qlonglong trimStartMillis,
    qlonglong trimEndMillis,
    rust::Vec<echo::desktop::EditSegmentWire>& destination
) {
    const auto segments =
        PlaybackAdjustmentProjection::editSegmentsFromQml(values, trimStartMillis, trimEndMillis);
    if (!segments.has_value()) {
        return false;
    }
    for (const auto& segment : *segments) {
        echo::desktop::EditSegmentWire wire;
        wire.source_start_millis = segment.source_start_millis;
        wire.source_end_millis = segment.source_end_millis;
        wire.state = static_cast<std::uint8_t>(segment.state);
        wire.gain_centibels = segment.gain_centibels;
        wire.fade_in_millis = segment.fade_in_millis;
        wire.fade_out_millis = segment.fade_out_millis;
        wire.fade_in_curve = static_cast<std::uint8_t>(segment.fade_in_curve);
        wire.fade_out_curve = static_cast<std::uint8_t>(segment.fade_out_curve);
        wire.gap_after_millis = segment.gap_after_millis;
        destination.push_back(wire);
    }
    return true;
}

QVariantList editSegmentsForQml(const rust::Vec<echo::desktop::EditSegmentWire>& segments) {
    QVariantList result;
    for (const auto& segment : segments) {
        QVariantMap value;
        value.insert(
            QStringLiteral("sourceStartMillis"),
            static_cast<qlonglong>(segment.source_start_millis)
        );
        value.insert(
            QStringLiteral("sourceEndMillis"),
            static_cast<qlonglong>(segment.source_end_millis)
        );
        value.insert(QStringLiteral("state"), static_cast<int>(segment.state));
        value.insert(QStringLiteral("gainCentibels"), static_cast<int>(segment.gain_centibels));
        value.insert(
            QStringLiteral("fadeInMillis"),
            static_cast<qlonglong>(segment.fade_in_millis)
        );
        value.insert(
            QStringLiteral("fadeOutMillis"),
            static_cast<qlonglong>(segment.fade_out_millis)
        );
        value.insert(QStringLiteral("fadeInCurve"), static_cast<int>(segment.fade_in_curve));
        value.insert(QStringLiteral("fadeOutCurve"), static_cast<int>(segment.fade_out_curve));
        value.insert(
            QStringLiteral("gapAfterMillis"),
            static_cast<qlonglong>(segment.gap_after_millis)
        );
        result.append(value);
    }
    return result;
}

bool appendEffectMasks(
    const QVariantList& values,
    const QVariantList& effectChain,
    qlonglong trimStartMillis,
    qlonglong trimEndMillis,
    rust::Vec<echo::desktop::EffectMaskWire>& destination
) {
    const auto masks = PlaybackAdjustmentProjection::effectMasksFromQml(
        values,
        trimStartMillis,
        trimEndMillis,
        effectChain
    );
    if (!masks.has_value()) {
        return false;
    }
    for (const auto& mask : *masks) {
        echo::desktop::EffectMaskWire wire;
        wire.start_millis = mask.start_millis;
        wire.end_millis = mask.end_millis;
        wire.feather_millis = static_cast<std::uint16_t>(mask.feather_millis);
        for (const auto node : mask.nodes) {
            wire.effect_nodes.push_back(static_cast<std::uint8_t>(node));
        }
        destination.push_back(std::move(wire));
    }
    return true;
}

QVariantList effectMasksForQml(const rust::Vec<echo::desktop::EffectMaskWire>& masks) {
    QVariantList result;
    for (const auto& mask : masks) {
        QVariantMap value;
        value.insert(QStringLiteral("startMillis"), static_cast<qlonglong>(mask.start_millis));
        value.insert(QStringLiteral("endMillis"), static_cast<qlonglong>(mask.end_millis));
        value.insert(QStringLiteral("featherMillis"), static_cast<int>(mask.feather_millis));
        QVariantList nodes;
        for (const std::uint8_t node : mask.effect_nodes) {
            nodes.append(static_cast<int>(node));
        }
        value.insert(QStringLiteral("effectNodes"), nodes);
        result.append(value);
    }
    return result;
}

QString processingComponentId(std::uint8_t value) {
    switch (value) {
    case 0:
        return QStringLiteral("lowCut");
    case 1:
        return QStringLiteral("restoration");
    case 2:
        return QStringLiteral("deHum");
    case 3:
        return QStringLiteral("deClick");
    case 4:
        return QStringLiteral("equalizer");
    case 5:
        return QStringLiteral("dynamics");
    case 6:
        return QStringLiteral("space");
    case 7:
        return QStringLiteral("master");
    case 8:
        return QStringLiteral("channelRepair");
    case 9:
        return QStringLiteral("sceneVfx");
    case 10:
        return QStringLiteral("delayVfx");
    case 11:
        return QStringLiteral("modulationVfx");
    case 12:
        return QStringLiteral("transformVfx");
    case 13:
        return QStringLiteral("digitalDegradeVfx");
    case 14:
        return QStringLiteral("driveVfx");
    case 15:
        return QStringLiteral("rotaryVfx");
    default:
        return {};
    }
}

bool appendProcessingComponents(const QVariantList& values, rust::Vec<std::uint8_t>& destination) {
    std::array<bool, 16> seen{};
    for (const QVariant& item : values) {
        const QString componentId = item.toString();
        int value = -1;
        if (componentId == QStringLiteral("lowCut")) {
            value = 0;
        } else if (componentId == QStringLiteral("restoration")) {
            value = 1;
        } else if (componentId == QStringLiteral("deHum")) {
            value = 2;
        } else if (componentId == QStringLiteral("deClick")) {
            value = 3;
        } else if (componentId == QStringLiteral("equalizer")) {
            value = 4;
        } else if (componentId == QStringLiteral("dynamics")) {
            value = 5;
        } else if (componentId == QStringLiteral("space")) {
            value = 6;
        } else if (componentId == QStringLiteral("master")) {
            value = 7;
        } else if (componentId == QStringLiteral("channelRepair")) {
            value = 8;
        } else if (componentId == QStringLiteral("sceneVfx")) {
            value = 9;
        } else if (componentId == QStringLiteral("delayVfx")) {
            value = 10;
        } else if (componentId == QStringLiteral("modulationVfx")) {
            value = 11;
        } else if (componentId == QStringLiteral("transformVfx")) {
            value = 12;
        } else if (componentId == QStringLiteral("digitalDegradeVfx")) {
            value = 13;
        } else if (componentId == QStringLiteral("driveVfx")) {
            value = 14;
        } else if (componentId == QStringLiteral("rotaryVfx")) {
            value = 15;
        }
        if (value < 0 || seen[static_cast<std::size_t>(value)]) {
            return false;
        }
        seen[static_cast<std::size_t>(value)] = true;
        destination.push_back(static_cast<std::uint8_t>(value));
    }
    return !destination.empty();
}

QVariantMap analysisStatusForQml(const echo::desktop::AnalysisStatusWire& wire) {
    QVariantMap status;
    status.insert(
        QStringLiteral("assetId"),
        QString::fromUtf8(wire.asset_id.data(), wire.asset_id.size())
    );
    status.insert(QStringLiteral("stage"), QString::fromUtf8(wire.stage.data(), wire.stage.size()));
    status.insert(QStringLiteral("state"), QString::fromUtf8(wire.state.data(), wire.state.size()));
    status.insert(
        QStringLiteral("recovery"),
        QString::fromUtf8(wire.recovery.data(), wire.recovery.size())
    );
    status.insert(QStringLiteral("progress"), static_cast<int>(wire.progress));
    status.insert(QStringLiteral("attempts"), static_cast<quint32>(wire.attempts));
    status.insert(
        QStringLiteral("errorCode"),
        QString::fromUtf8(wire.error_code.data(), wire.error_code.size())
    );
    status.insert(
        QStringLiteral("runtimeJobId"),
        QString::fromUtf8(wire.runtime_job_id.data(), wire.runtime_job_id.size())
    );
    status.insert(
        QStringLiteral("contractVersion"),
        QString::fromUtf8(wire.contract_version.data(), wire.contract_version.size())
    );
    return status;
}

} // namespace

DesktopBackend::DesktopBackend(rust::Box<echo::desktop::LibrarySession> session, QObject* parent) :
    QObject(parent), session_(std::move(session)) {
    analysisRefreshTimer_.setInterval(250);
    analysisRefreshTimer_.setTimerType(Qt::CoarseTimer);
    connect(&analysisRefreshTimer_, &QTimer::timeout, this, [this]() {
        const quint64 revision = session_->session_worker_state_revision();
        if (revision == 0 || revision == workerStateRevision_) {
            return;
        }
        workerStateRevision_ = revision;
        emit jobsChanged();
    });
}

void DesktopBackend::refresh() {
    emit assetsChanged();
}

QVariantList DesktopBackend::listAssets() const {
    QVariantList list;
    const auto assets = session_->session_list_assets();
    for (const auto& asset : assets) {
        QVariantMap entry;
        entry.insert(QStringLiteral("id"), QString::fromUtf8(asset.id.data(), asset.id.size()));
        entry.insert(
            QStringLiteral("path"),
            QString::fromUtf8(asset.path.data(), asset.path.size())
        );
        entry.insert(
            QStringLiteral("codec"),
            QString::fromUtf8(asset.codec.data(), asset.codec.size())
        );
        entry.insert(
            QStringLiteral("durationMillis"),
            static_cast<qlonglong>(asset.duration_millis)
        );
        entry.insert(
            QStringLiteral("importedAtMillis"),
            static_cast<qlonglong>(asset.imported_at_millis)
        );
        entry.insert(
            QStringLiteral("recordedAtMillis"),
            static_cast<qlonglong>(asset.recorded_at_millis)
        );
        entry.insert(QStringLiteral("maxLevel"), static_cast<int>(asset.max_level));
        entry.insert(
            QStringLiteral("pathStatus"),
            QString::fromUtf8(asset.path_status.data(), asset.path_status.size())
        );
        entry.insert(
            QStringLiteral("soundCaption"),
            QString::fromUtf8(asset.sound_caption.data(), asset.sound_caption.size())
        );
        entry.insert(
            QStringLiteral("summary"),
            QString::fromUtf8(asset.summary.data(), asset.summary.size())
        );
        entry.insert(
            QStringLiteral("eventType"),
            QString::fromUtf8(asset.event_type.data(), asset.event_type.size())
        );
        entry.insert(
            QStringLiteral("mood"),
            QString::fromUtf8(asset.mood.data(), asset.mood.size())
        );
        entry.insert(
            QStringLiteral("textPreview"),
            QString::fromUtf8(asset.text_preview.data(), asset.text_preview.size())
        );
        entry.insert(
            QStringLiteral("language"),
            QString::fromUtf8(asset.language.data(), asset.language.size())
        );
        entry.insert(
            QStringLiteral("modelSoundCaption"),
            QString::fromUtf8(asset.model_sound_caption.data(), asset.model_sound_caption.size())
        );
        entry.insert(
            QStringLiteral("modelSummary"),
            QString::fromUtf8(asset.model_summary.data(), asset.model_summary.size())
        );
        entry.insert(
            QStringLiteral("modelEventType"),
            QString::fromUtf8(asset.model_event_type.data(), asset.model_event_type.size())
        );
        entry.insert(
            QStringLiteral("modelMood"),
            QString::fromUtf8(asset.model_mood.data(), asset.model_mood.size())
        );
        entry.insert(
            QStringLiteral("modelTextPreview"),
            QString::fromUtf8(asset.model_text_preview.data(), asset.model_text_preview.size())
        );
        entry.insert(
            QStringLiteral("modelLanguage"),
            QString::fromUtf8(asset.model_language.data(), asset.model_language.size())
        );
        entry.insert(
            QStringLiteral("metadataCalibrationRevision"),
            static_cast<qlonglong>(asset.metadata_calibration_revision)
        );
        QVariantList modelKeywords;
        for (const auto& keyword : asset.model_keywords) {
            modelKeywords.append(QString::fromUtf8(keyword.data(), keyword.size()));
        }
        entry.insert(QStringLiteral("modelKeywords"), modelKeywords);
        QVariantList calibratedFields;
        for (const auto& field : asset.calibrated_fields) {
            calibratedFields.append(QString::fromUtf8(field.data(), field.size()));
        }
        entry.insert(QStringLiteral("calibratedFields"), calibratedFields);
        entry.insert(
            QStringLiteral("analysisStage"),
            QString::fromUtf8(asset.analysis_stage.data(), asset.analysis_stage.size())
        );
        entry.insert(
            QStringLiteral("analysisState"),
            QString::fromUtf8(asset.analysis_state.data(), asset.analysis_state.size())
        );
        entry.insert(
            QStringLiteral("analysisRecovery"),
            QString::fromUtf8(asset.analysis_recovery.data(), asset.analysis_recovery.size())
        );
        entry.insert(
            QStringLiteral("analysisErrorCode"),
            QString::fromUtf8(asset.analysis_error_code.data(), asset.analysis_error_code.size())
        );
        entry.insert(QStringLiteral("analysisProgress"), static_cast<int>(asset.analysis_progress));
        entry.insert(
            QStringLiteral("analysisAttempts"),
            static_cast<quint32>(asset.analysis_attempts)
        );
        entry.insert(QStringLiteral("liked"), asset.liked);
        entry.insert(QStringLiteral("rating"), static_cast<int>(asset.rating));
        entry.insert(
            QStringLiteral("lastListenedAtMillis"),
            static_cast<qlonglong>(asset.last_listened_at_millis)
        );
        entry.insert(
            QStringLiteral("resumePositionMillis"),
            static_cast<qlonglong>(asset.resume_position_millis)
        );
        entry.insert(
            QStringLiteral("adjustmentRevision"),
            static_cast<qlonglong>(asset.adjustment_revision)
        );
        entry.insert(
            QStringLiteral("trimStartMillis"),
            static_cast<qlonglong>(asset.trim_start_millis)
        );
        entry.insert(
            QStringLiteral("trimEndMillis"),
            static_cast<qlonglong>(asset.trim_end_millis)
        );
        entry.insert(QStringLiteral("fadeInMillis"), static_cast<qlonglong>(asset.fade_in_millis));
        entry.insert(
            QStringLiteral("fadeOutMillis"),
            static_cast<qlonglong>(asset.fade_out_millis)
        );
        entry.insert(QStringLiteral("fadeInCurve"), static_cast<int>(asset.fade_in_curve));
        entry.insert(QStringLiteral("fadeOutCurve"), static_cast<int>(asset.fade_out_curve));
        entry.insert(QStringLiteral("gainCentibels"), static_cast<int>(asset.gain_centibels));
        entry.insert(QStringLiteral("lowCutHertz"), static_cast<int>(asset.low_cut_hertz));
        entry.insert(QStringLiteral("restorationEnabled"), asset.restoration_enabled);
        entry.insert(QStringLiteral("dePlosiveEnabled"), asset.de_plosive_enabled);
        entry.insert(
            QStringLiteral("dePlosiveFrequencyHertz"),
            static_cast<int>(asset.de_plosive_frequency_hertz)
        );
        entry.insert(
            QStringLiteral("dePlosiveSensitivityPercent"),
            static_cast<int>(asset.de_plosive_sensitivity_percent)
        );
        entry.insert(
            QStringLiteral("dePlosiveReductionCentibels"),
            static_cast<int>(asset.de_plosive_reduction_centibels)
        );
        entry.insert(
            QStringLiteral("dePlosiveReleaseMillis"),
            static_cast<int>(asset.de_plosive_release_millis)
        );
        entry.insert(QStringLiteral("noiseReductionEnabled"), asset.noise_reduction_enabled);
        entry.insert(
            QStringLiteral("noiseReductionCentibels"),
            static_cast<int>(asset.noise_reduction_centibels)
        );
        entry.insert(
            QStringLiteral("noiseReductionSensitivityPercent"),
            static_cast<int>(asset.noise_reduction_sensitivity_percent)
        );
        entry.insert(
            QStringLiteral("noiseReductionSmoothingMillis"),
            static_cast<int>(asset.noise_reduction_smoothing_millis)
        );
        entry.insert(QStringLiteral("deEsserEnabled"), asset.de_esser_enabled);
        entry.insert(
            QStringLiteral("deEsserFrequencyHertz"),
            static_cast<int>(asset.de_esser_frequency_hertz)
        );
        entry.insert(
            QStringLiteral("deEsserThresholdCentibels"),
            static_cast<int>(asset.de_esser_threshold_centibels)
        );
        entry.insert(
            QStringLiteral("deEsserReductionCentibels"),
            static_cast<int>(asset.de_esser_reduction_centibels)
        );
        entry.insert(QStringLiteral("deHumEnabled"), asset.de_hum_enabled);
        entry.insert(
            QStringLiteral("deHumFundamentalHertz"),
            static_cast<int>(asset.de_hum_fundamental_hertz)
        );
        entry.insert(
            QStringLiteral("deHumHarmonicCount"),
            static_cast<int>(asset.de_hum_harmonic_count)
        );
        entry.insert(
            QStringLiteral("deHumQualityTenths"),
            static_cast<int>(asset.de_hum_quality_tenths)
        );
        entry.insert(
            QStringLiteral("deHumDepthCentibels"),
            static_cast<int>(asset.de_hum_depth_centibels)
        );
        entry.insert(QStringLiteral("deClickEnabled"), asset.de_click_enabled);
        entry.insert(
            QStringLiteral("deClickSensitivityPercent"),
            static_cast<int>(asset.de_click_sensitivity_percent)
        );
        entry.insert(
            QStringLiteral("deClickMaximumClickMicroseconds"),
            static_cast<int>(asset.de_click_maximum_click_microseconds)
        );
        entry.insert(
            QStringLiteral("deClickRepairPercent"),
            static_cast<int>(asset.de_click_repair_percent)
        );
        entry.insert(QStringLiteral("channelRepairEnabled"), asset.channel_repair_enabled);
        entry.insert(QStringLiteral("channelRepairInvertLeft"), asset.channel_repair_invert_left);
        entry.insert(QStringLiteral("channelRepairInvertRight"), asset.channel_repair_invert_right);
        entry.insert(
            QStringLiteral("channelRepairSwapChannels"),
            asset.channel_repair_swap_channels
        );
        entry.insert(
            QStringLiteral("channelRepairMonoFoldDown"),
            asset.channel_repair_mono_fold_down
        );
        entry.insert(
            QStringLiteral("channelRepairBalancePercent"),
            static_cast<int>(asset.channel_repair_balance_percent)
        );
        entry.insert(QStringLiteral("equalizerEnabled"), asset.equalizer_enabled);
        entry.insert(QStringLiteral("equalizerBands"), equalizerBandsForQml(asset.equalizer_bands));
        entry.insert(QStringLiteral("compressorEnabled"), asset.compressor_enabled);
        entry.insert(
            QStringLiteral("compressorThresholdCentibels"),
            static_cast<int>(asset.compressor_threshold_centibels)
        );
        entry.insert(
            QStringLiteral("compressorRatioTenths"),
            static_cast<int>(asset.compressor_ratio_tenths)
        );
        entry.insert(
            QStringLiteral("compressorAttackMillis"),
            static_cast<int>(asset.compressor_attack_millis)
        );
        entry.insert(
            QStringLiteral("compressorReleaseMillis"),
            static_cast<int>(asset.compressor_release_millis)
        );
        entry.insert(
            QStringLiteral("compressorMakeupCentibels"),
            static_cast<int>(asset.compressor_makeup_centibels)
        );
        entry.insert(QStringLiteral("reverbCharacter"), static_cast<int>(asset.reverb_character));
        entry.insert(QStringLiteral("reverbEnabled"), asset.reverb_enabled);
        entry.insert(
            QStringLiteral("reverbMixPercent"),
            static_cast<int>(asset.reverb_mix_percent)
        );
        entry.insert(
            QStringLiteral("reverbPreDelayMillis"),
            static_cast<int>(asset.reverb_pre_delay_millis)
        );
        entry.insert(
            QStringLiteral("reverbDecayMillis"),
            static_cast<int>(asset.reverb_decay_millis)
        );
        entry.insert(
            QStringLiteral("reverbSizePercent"),
            static_cast<int>(asset.reverb_size_percent)
        );
        entry.insert(
            QStringLiteral("reverbDampingPercent"),
            static_cast<int>(asset.reverb_damping_percent)
        );
        entry.insert(
            QStringLiteral("reverbLowCutHertz"),
            static_cast<int>(asset.reverb_low_cut_hertz)
        );
        entry.insert(
            QStringLiteral("reverbHighCutHertz"),
            static_cast<int>(asset.reverb_high_cut_hertz)
        );
        entry.insert(QStringLiteral("reverbDuckingEnabled"), asset.reverb_ducking_enabled);
        entry.insert(
            QStringLiteral("reverbDuckingAmountPercent"),
            static_cast<int>(asset.reverb_ducking_amount_percent)
        );
        entry.insert(
            QStringLiteral("reverbDuckingAttackMillis"),
            static_cast<int>(asset.reverb_ducking_attack_millis)
        );
        entry.insert(
            QStringLiteral("reverbDuckingReleaseMillis"),
            static_cast<int>(asset.reverb_ducking_release_millis)
        );
        const QString impulse_import_id = QString::fromUtf8(
            asset.impulse_response_import_id.data(),
            asset.impulse_response_import_id.size()
        );
        entry.insert(QStringLiteral("spaceMode"), static_cast<int>(asset.space_mode));
        entry.insert(QStringLiteral("impulseResponseImportId"), impulse_import_id);
        entry.insert(
            QStringLiteral("impulseResponseSourceHash"),
            QString::fromUtf8(
                asset.impulse_response_source_hash.data(),
                asset.impulse_response_source_hash.size()
            )
        );
        entry.insert(
            QStringLiteral("impulseResponsePreparedHash"),
            QString::fromUtf8(
                asset.impulse_response_prepared_hash.data(),
                asset.impulse_response_prepared_hash.size()
            )
        );
        entry.insert(
            QStringLiteral("impulseResponsePreparedPath"),
            QString::fromUtf8(
                asset.impulse_response_prepared_path.data(),
                asset.impulse_response_prepared_path.size()
            )
        );
        entry.insert(
            QStringLiteral("convolutionMixPercent"),
            static_cast<int>(asset.convolution_mix_percent)
        );
        entry.insert(
            QStringLiteral("convolutionWetGainCentibels"),
            static_cast<int>(asset.convolution_wet_gain_centibels)
        );
        entry.insert(QStringLiteral("creativeVfx"), creativeVfxForQml(asset.creative_vfx_json));
        entry.insert(QStringLiteral("limiterEnabled"), asset.limiter_enabled);
        entry.insert(
            QStringLiteral("limiterCeilingCentibels"),
            static_cast<int>(asset.limiter_ceiling_centibels)
        );
        entry.insert(
            QStringLiteral("limiterReleaseMillis"),
            static_cast<int>(asset.limiter_release_millis)
        );
        entry.insert(QStringLiteral("effectChain"), effectChainForQml(asset.effect_chain));
        entry.insert(QStringLiteral("editSegments"), editSegmentsForQml(asset.edit_segments));
        entry.insert(QStringLiteral("effectMasks"), effectMasksForQml(asset.effect_masks));
        entry.insert(
            QStringLiteral("containerFormat"),
            QString::fromUtf8(asset.container_format.data(), asset.container_format.size())
        );
        entry.insert(QStringLiteral("sampleRate"), static_cast<int>(asset.sample_rate));
        entry.insert(QStringLiteral("channelCount"), static_cast<int>(asset.channel_count));
        entry.insert(
            QStringLiteral("sourceTitle"),
            QString::fromUtf8(asset.source_title.data(), asset.source_title.size())
        );
        entry.insert(
            QStringLiteral("sourceLocation"),
            QString::fromUtf8(asset.source_location.data(), asset.source_location.size())
        );
        entry.insert(
            QStringLiteral("sourceCreatedAt"),
            QString::fromUtf8(asset.source_created_at.data(), asset.source_created_at.size())
        );
        QVariantList keywords;
        for (const auto& keyword : asset.keywords) {
            keywords.append(QString::fromUtf8(keyword.data(), keyword.size()));
        }
        entry.insert(QStringLiteral("keywords"), keywords);
        list.append(entry);
    }
    return list;
}

QVariantList DesktopBackend::listImpulseResponses() const {
    QVariantList result;
    try {
        for (const auto& wire : session_->session_impulse_responses()) {
            result.append(impulseResponseForQml(wire));
        }
    } catch (const rust::Error& error) {
        qWarning("cannot list impulse responses: %s", error.what());
    }
    return result;
}

QVariantMap DesktopBackend::importImpulseResponse(
    const QString& sourcePath,
    const QString& displayName,
    const QString& creator,
    const QString& sourceUrl,
    const QString& attribution,
    const QString& rightsKind,
    const QString& spdxExpression,
    const QString& licenseUrl
) const {
    try {
        const auto wire = session_->session_import_impulse_response(
            sourcePath.toStdString(),
            displayName.toStdString(),
            creator.toStdString(),
            sourceUrl.toStdString(),
            attribution.toStdString(),
            rightsKind.toStdString(),
            spdxExpression.toStdString(),
            licenseUrl.toStdString()
        );
        return impulseResponseForQml(wire);
    } catch (const rust::Error& error) {
        qWarning("cannot import impulse response: %s", error.what());
        return {{QStringLiteral("error"), QString::fromUtf8(error.what())}};
    }
}

QVariantMap DesktopBackend::importImpulseResponseWithLayout(
    const QString& sourcePath,
    const QString& preparationLayout,
    const QString& displayName,
    const QString& creator,
    const QString& sourceUrl,
    const QString& attribution,
    const QString& rightsKind,
    const QString& spdxExpression,
    const QString& licenseUrl
) const {
    try {
        const auto wire = session_->session_import_impulse_response_with_layout(
            sourcePath.toStdString(),
            preparationLayout.toStdString(),
            displayName.toStdString(),
            creator.toStdString(),
            sourceUrl.toStdString(),
            attribution.toStdString(),
            rightsKind.toStdString(),
            spdxExpression.toStdString(),
            licenseUrl.toStdString()
        );
        return impulseResponseForQml(wire);
    } catch (const rust::Error& error) {
        qWarning("cannot import impulse response with layout: %s", error.what());
        return {{QStringLiteral("error"), QString::fromUtf8(error.what())}};
    }
}

QVariantList DesktopBackend::listKeywordFacets() const {
    QVariantList list;
    try {
        const auto facets = session_->session_keyword_facets();
        for (const auto& facet : facets) {
            QVariantMap entry;
            entry.insert(
                QStringLiteral("key"),
                QString::fromUtf8(facet.key.data(), facet.key.size())
            );
            entry.insert(
                QStringLiteral("label"),
                QString::fromUtf8(facet.label.data(), facet.label.size())
            );
            entry.insert(QStringLiteral("count"), static_cast<qulonglong>(facet.count));
            list.append(entry);
        }
    } catch (const rust::Error& error) {
        qWarning("cannot list contextual keyword facets: %s", error.what());
    }
    return list;
}

QVariantList DesktopBackend::listSmartAlbums() const {
    QVariantList list;
    try {
        const auto albums = session_->session_smart_albums();
        for (const auto& album : albums) {
            QVariantMap entry;
            entry.insert(
                QStringLiteral("key"),
                QString::fromUtf8(album.key.data(), album.key.size())
            );
            entry.insert(
                QStringLiteral("label"),
                QString::fromUtf8(album.label.data(), album.label.size())
            );
            entry.insert(
                QStringLiteral("facet"),
                QString::fromUtf8(album.facet.data(), album.facet.size())
            );
            entry.insert(
                QStringLiteral("evidence"),
                QString::fromUtf8(album.evidence.data(), album.evidence.size())
            );
            entry.insert(QStringLiteral("count"), static_cast<qulonglong>(album.count));
            QVariantList memberIds;
            for (const auto& memberId : album.member_asset_ids) {
                memberIds.append(QString::fromUtf8(memberId.data(), memberId.size()));
            }
            entry.insert(QStringLiteral("memberIds"), memberIds);
            list.append(entry);
        }
    } catch (const rust::Error& error) {
        qWarning("cannot list smart album candidates: %s", error.what());
    }
    return list;
}

QVariantList DesktopBackend::listUserAlbums() const {
    QVariantList list;
    try {
        const auto albums = session_->session_user_albums();
        for (const auto& album : albums) {
            QVariantMap entry;
            entry.insert(QStringLiteral("id"), static_cast<qlonglong>(album.id));
            entry.insert(
                QStringLiteral("name"),
                QString::fromUtf8(album.name.data(), album.name.size())
            );
            entry.insert(
                QStringLiteral("coverAssetId"),
                QString::fromUtf8(album.cover_asset_id.data(), album.cover_asset_id.size())
            );
            entry.insert(QStringLiteral("count"), static_cast<qulonglong>(album.count));
            entry.insert(
                QStringLiteral("createdAtMillis"),
                static_cast<qlonglong>(album.created_at_millis)
            );
            entry.insert(
                QStringLiteral("updatedAtMillis"),
                static_cast<qlonglong>(album.updated_at_millis)
            );
            QVariantList memberIds;
            for (const auto& memberId : album.member_asset_ids) {
                memberIds.append(QString::fromUtf8(memberId.data(), memberId.size()));
            }
            entry.insert(QStringLiteral("memberIds"), memberIds);
            list.append(entry);
        }
    } catch (const rust::Error& error) {
        qWarning("cannot list user albums: %s", error.what());
    }
    return list;
}

QVariantMap DesktopBackend::revisitSnapshot() const {
    QVariantMap result;
    const auto idsForQml = [](const rust::Vec<rust::String>& values) {
        QVariantList ids;
        ids.reserve(static_cast<qsizetype>(values.size()));
        for (const auto& value : values) {
            ids.append(QString::fromUtf8(value.data(), value.size()));
        }
        return ids;
    };
    try {
        const auto snapshot =
            session_->session_revisit_snapshot(QDateTime::currentMSecsSinceEpoch());
        result.insert(
            QStringLiteral("continueListeningAssetIds"),
            idsForQml(snapshot.continue_listening)
        );
        result.insert(
            QStringLiteral("recentlyListenedAssetIds"),
            idsForQml(snapshot.recently_listened)
        );
        result.insert(QStringLiteral("onThisDayAssetIds"), idsForQml(snapshot.on_this_day));
        result.insert(QStringLiteral("recentlyAddedAssetIds"), idsForQml(snapshot.recently_added));
    } catch (const rust::Error& error) {
        qWarning("cannot project Revisit home: %s", error.what());
    }
    return result;
}

QVariantList DesktopBackend::listProcessingRecipes() const {
    QVariantList list;
    try {
        const auto recipes = session_->session_processing_recipes();
        for (const auto& recipe : recipes) {
            QVariantMap entry;
            entry.insert(
                QStringLiteral("id"),
                QString::fromUtf8(recipe.id.data(), recipe.id.size())
            );
            entry.insert(
                QStringLiteral("name"),
                QString::fromUtf8(recipe.name.data(), recipe.name.size())
            );
            entry.insert(
                QStringLiteral("revisionId"),
                QString::fromUtf8(recipe.revision_id.data(), recipe.revision_id.size())
            );
            entry.insert(
                QStringLiteral("revisionNumber"),
                static_cast<qulonglong>(recipe.revision_number)
            );
            entry.insert(
                QStringLiteral("updatedAtMillis"),
                static_cast<qlonglong>(recipe.updated_at_millis)
            );
            QVariantList components;
            for (const std::uint8_t value : recipe.components) {
                const QString componentId = processingComponentId(value);
                if (!componentId.isEmpty()) {
                    components.append(componentId);
                }
            }
            entry.insert(QStringLiteral("components"), components);
            list.append(entry);
        }
    } catch (const rust::Error& error) {
        qWarning("cannot list processing recipes: %s", error.what());
    }
    return list;
}

QVariantList DesktopBackend::listProcessingRecipeHistory() const {
    QVariantList list;
    try {
        const auto history = session_->session_processing_recipe_history();
        for (const auto& item : history) {
            QVariantMap entry;
            entry.insert(
                QStringLiteral("batchId"),
                QString::fromUtf8(item.batch_id.data(), item.batch_id.size())
            );
            entry.insert(
                QStringLiteral("recipeId"),
                QString::fromUtf8(item.recipe_id.data(), item.recipe_id.size())
            );
            entry.insert(
                QStringLiteral("recipeName"),
                QString::fromUtf8(item.recipe_name.data(), item.recipe_name.size())
            );
            entry.insert(
                QStringLiteral("recipeRevisionId"),
                QString::fromUtf8(item.recipe_revision_id.data(), item.recipe_revision_id.size())
            );
            entry.insert(
                QStringLiteral("recipeRevisionNumber"),
                static_cast<qulonglong>(item.recipe_revision_number)
            );
            entry.insert(
                QStringLiteral("mergeMode"),
                QString::fromUtf8(item.merge_mode.data(), item.merge_mode.size())
            );
            entry.insert(QStringLiteral("targetCount"), static_cast<qulonglong>(item.target_count));
            entry.insert(
                QStringLiteral("updatedCount"),
                static_cast<qulonglong>(item.updated_count)
            );
            entry.insert(
                QStringLiteral("unchangedCount"),
                static_cast<qulonglong>(item.unchanged_count)
            );
            entry.insert(QStringLiteral("failedCount"), static_cast<qulonglong>(item.failed_count));
            entry.insert(
                QStringLiteral("createdAtMillis"),
                static_cast<qlonglong>(item.created_at_millis)
            );
            entry.insert(QStringLiteral("reverted"), item.reverted);
            entry.insert(
                QStringLiteral("revertId"),
                QString::fromUtf8(item.revert_id.data(), item.revert_id.size())
            );
            entry.insert(
                QStringLiteral("restoredCount"),
                static_cast<qulonglong>(item.restored_count)
            );
            entry.insert(
                QStringLiteral("revertUnchangedCount"),
                static_cast<qulonglong>(item.revert_unchanged_count)
            );
            entry.insert(
                QStringLiteral("conflictCount"),
                static_cast<qulonglong>(item.conflict_count)
            );
            entry.insert(
                QStringLiteral("revertFailedCount"),
                static_cast<qulonglong>(item.revert_failed_count)
            );
            entry.insert(
                QStringLiteral("revertedAtMillis"),
                static_cast<qlonglong>(item.reverted_at_millis)
            );
            list.append(entry);
        }
    } catch (const rust::Error& error) {
        qWarning("cannot list processing recipe history: %s", error.what());
    }
    return list;
}

QString DesktopBackend::createProcessingRecipe(
    const QString& name,
    const QString& sourceAssetId,
    const QVariantList& componentIds
) {
    rust::Vec<std::uint8_t> components;
    if (!appendProcessingComponents(componentIds, components)) {
        qWarning("processing recipe components are outside the supported contract");
        return {};
    }
    try {
        const auto componentSlice =
            rust::Slice<const std::uint8_t>(components.data(), components.size());
        const auto recipeId = session_->session_create_processing_recipe(
            name.toStdString(),
            sourceAssetId.toStdString(),
            componentSlice
        );
        emit processingRecipesChanged();
        return QString::fromUtf8(recipeId.data(), recipeId.size());
    } catch (const rust::Error& error) {
        qWarning("cannot create processing recipe: %s", error.what());
        return {};
    }
}

bool DesktopBackend::renameProcessingRecipe(const QString& recipeId, const QString& name) {
    try {
        session_->session_rename_processing_recipe(recipeId.toStdString(), name.toStdString());
        emit processingRecipesChanged();
        return true;
    } catch (const rust::Error& error) {
        qWarning("cannot rename processing recipe: %s", error.what());
        return false;
    }
}

qlonglong DesktopBackend::updateProcessingRecipe(
    const QString& recipeId,
    const QString& sourceAssetId,
    const QVariantList& componentIds
) {
    rust::Vec<std::uint8_t> components;
    if (!appendProcessingComponents(componentIds, components)) {
        qWarning("processing recipe components are outside the supported contract");
        return 0;
    }
    try {
        const auto componentSlice =
            rust::Slice<const std::uint8_t>(components.data(), components.size());
        const auto revision = session_->session_update_processing_recipe(
            recipeId.toStdString(),
            sourceAssetId.toStdString(),
            componentSlice
        );
        emit processingRecipesChanged();
        return static_cast<qlonglong>(revision);
    } catch (const rust::Error& error) {
        qWarning("cannot update processing recipe: %s", error.what());
        return 0;
    }
}

bool DesktopBackend::archiveProcessingRecipe(const QString& recipeId) {
    try {
        session_->session_archive_processing_recipe(recipeId.toStdString());
        emit processingRecipesChanged();
        return true;
    } catch (const rust::Error& error) {
        qWarning("cannot archive processing recipe: %s", error.what());
        return false;
    }
}

QVariantMap DesktopBackend::applyProcessingRecipe(
    const QString& recipeId,
    const QVariantList& targetAssetIds,
    const QString& mergeMode
) {
    QVariantMap result;
    if (targetAssetIds.isEmpty()
        || (mergeMode != QStringLiteral("merge") && mergeMode != QStringLiteral("replace"))) {
        qWarning("processing recipe application is outside the supported contract");
        return result;
    }
    rust::Vec<rust::String> targets;
    targets.reserve(static_cast<std::size_t>(targetAssetIds.size()));
    for (const QVariant& target : targetAssetIds) {
        targets.push_back(target.toString().toStdString());
    }
    try {
        const auto targetSlice = rust::Slice<const rust::String>(targets.data(), targets.size());
        const auto receipt = session_->session_apply_processing_recipe(
            recipeId.toStdString(),
            targetSlice,
            mergeMode == QStringLiteral("replace") ? 1 : 0
        );
        result.insert(
            QStringLiteral("batchId"),
            QString::fromUtf8(receipt.batch_id.data(), receipt.batch_id.size())
        );
        result.insert(
            QStringLiteral("recipeRevisionId"),
            QString::fromUtf8(receipt.recipe_revision_id.data(), receipt.recipe_revision_id.size())
        );
        result.insert(
            QStringLiteral("updatedCount"),
            static_cast<qulonglong>(receipt.updated_count)
        );
        result.insert(
            QStringLiteral("unchangedCount"),
            static_cast<qulonglong>(receipt.unchanged_count)
        );
        result.insert(QStringLiteral("failedCount"), static_cast<qulonglong>(receipt.failed_count));
        QVariantList targetResults;
        for (const auto& target : receipt.results) {
            QVariantMap targetResult;
            targetResult.insert(
                QStringLiteral("assetId"),
                QString::fromUtf8(target.asset_id.data(), target.asset_id.size())
            );
            targetResult.insert(
                QStringLiteral("outcome"),
                QString::fromUtf8(target.outcome.data(), target.outcome.size())
            );
            targetResult.insert(
                QStringLiteral("adjustmentRevision"),
                static_cast<qlonglong>(target.adjustment_revision)
            );
            targetResult.insert(
                QStringLiteral("error"),
                QString::fromUtf8(target.error.data(), target.error.size())
            );
            targetResults.append(targetResult);
        }
        result.insert(QStringLiteral("results"), targetResults);
        emit assetsChanged();
    } catch (const rust::Error& error) {
        qWarning("cannot apply processing recipe: %s", error.what());
    }
    return result;
}

QVariantMap DesktopBackend::revertProcessingRecipeApplication(const QString& batchId) {
    QVariantMap result;
    try {
        const auto receipt =
            session_->session_revert_processing_recipe_application(batchId.toStdString());
        result.insert(
            QStringLiteral("revertId"),
            QString::fromUtf8(receipt.revert_id.data(), receipt.revert_id.size())
        );
        result.insert(
            QStringLiteral("applicationBatchId"),
            QString::fromUtf8(
                receipt.application_batch_id.data(),
                receipt.application_batch_id.size()
            )
        );
        result.insert(
            QStringLiteral("restoredCount"),
            static_cast<qulonglong>(receipt.restored_count)
        );
        result.insert(
            QStringLiteral("unchangedCount"),
            static_cast<qulonglong>(receipt.unchanged_count)
        );
        result.insert(
            QStringLiteral("conflictCount"),
            static_cast<qulonglong>(receipt.conflict_count)
        );
        result.insert(QStringLiteral("failedCount"), static_cast<qulonglong>(receipt.failed_count));
        QVariantList targetResults;
        for (const auto& target : receipt.results) {
            QVariantMap targetResult;
            targetResult.insert(
                QStringLiteral("assetId"),
                QString::fromUtf8(target.asset_id.data(), target.asset_id.size())
            );
            targetResult.insert(
                QStringLiteral("outcome"),
                QString::fromUtf8(target.outcome.data(), target.outcome.size())
            );
            targetResult.insert(
                QStringLiteral("adjustmentRevision"),
                static_cast<qlonglong>(target.adjustment_revision)
            );
            targetResult.insert(
                QStringLiteral("error"),
                QString::fromUtf8(target.error.data(), target.error.size())
            );
            targetResults.append(targetResult);
        }
        result.insert(QStringLiteral("results"), targetResults);
        emit assetsChanged();
    } catch (const rust::Error& error) {
        qWarning("cannot revert processing recipe application: %s", error.what());
    }
    return result;
}

qlonglong DesktopBackend::createUserAlbum(const QString& name, const QVariantList& memberIds) {
    rust::Vec<rust::String> members;
    members.reserve(static_cast<std::size_t>(memberIds.size()));
    for (const auto& memberId : memberIds) {
        members.push_back(memberId.toString().toStdString());
    }
    try {
        const auto memberSlice = rust::Slice<const rust::String>(members.data(), members.size());
        const auto albumId = session_->session_create_user_album(name.toStdString(), memberSlice);
        emit albumsChanged();
        return static_cast<qlonglong>(albumId);
    } catch (const rust::Error& error) {
        qWarning("cannot create user album: %s", error.what());
        return -1;
    }
}

bool DesktopBackend::renameUserAlbum(qlonglong albumId, const QString& name) {
    try {
        session_->session_rename_user_album(static_cast<std::int64_t>(albumId), name.toStdString());
        emit albumsChanged();
        return true;
    } catch (const rust::Error& error) {
        qWarning("cannot rename user album: %s", error.what());
        return false;
    }
}

bool DesktopBackend::deleteUserAlbum(qlonglong albumId) {
    try {
        session_->session_delete_user_album(static_cast<std::int64_t>(albumId));
        emit albumsChanged();
        return true;
    } catch (const rust::Error& error) {
        qWarning("cannot delete user album: %s", error.what());
        return false;
    }
}

bool DesktopBackend::setUserAlbumMembership(
    qlonglong albumId,
    const QString& assetId,
    bool included
) {
    try {
        const bool changed = session_->session_set_user_album_membership(
            static_cast<std::int64_t>(albumId),
            assetId.toStdString(),
            included
        );
        if (changed) {
            emit albumsChanged();
        }
        return true;
    } catch (const rust::Error& error) {
        qWarning("cannot update user album membership: %s", error.what());
        return false;
    }
}

bool DesktopBackend::setAssetAffinity(const QString& id, bool liked, int rating) {
    if (rating < 0 || rating > 5) {
        qWarning("asset rating is outside zero to five");
        return false;
    }
    try {
        session_->session_set_asset_affinity(
            id.toStdString(),
            liked,
            static_cast<std::uint8_t>(rating)
        );
        emit assetsChanged();
        return true;
    } catch (const rust::Error& error) {
        qWarning("cannot update affinity for %s: %s", qPrintable(id), error.what());
        return false;
    }
}

QVariantMap DesktopBackend::calibrateAssetMetadata(
    const QString& id,
    const QString& soundCaption,
    const QString& summary,
    const QString& eventType,
    const QString& mood,
    const QVariantList& keywords,
    const QString& transcriptText,
    const QString& language,
    const QVariantList& calibratedFields
) {
    QJsonArray encodedKeywords;
    for (const auto& keyword : keywords) {
        encodedKeywords.append(keyword.toString());
    }
    const auto keywordsJson = QJsonDocument(encodedKeywords).toJson(QJsonDocument::Compact);
    QJsonArray encodedCalibratedFields;
    for (const auto& field : calibratedFields) {
        encodedCalibratedFields.append(field.toString());
    }
    const auto calibratedFieldsJson =
        QJsonDocument(encodedCalibratedFields).toJson(QJsonDocument::Compact);
    try {
        const auto revision = session_->session_calibrate_asset_metadata(
            id.toStdString(),
            soundCaption.toStdString(),
            summary.toStdString(),
            eventType.toStdString(),
            mood.toStdString(),
            keywordsJson.toStdString(),
            transcriptText.toStdString(),
            language.toStdString(),
            calibratedFieldsJson.toStdString()
        );
        emit assetsChanged();
        return {
            {QStringLiteral("ok"), true},
            {QStringLiteral("revision"), static_cast<qlonglong>(revision)},
            {QStringLiteral("error"), QString()},
        };
    } catch (const rust::Error& error) {
        qWarning("metadata calibration failed for %s: %s", qPrintable(id), error.what());
        return {
            {QStringLiteral("ok"), false},
            {QStringLiteral("revision"), 0},
            {QStringLiteral("error"), QString::fromUtf8(error.what())},
        };
    }
}

QVariantMap DesktopBackend::recordListeningProgress(
    const QString& id,
    qlonglong positionMillis,
    qlonglong playbackStartMillis,
    qlonglong playbackEndMillis
) {
    if (positionMillis < 0 || playbackStartMillis < 0 || playbackEndMillis <= playbackStartMillis
        || positionMillis < playbackStartMillis || positionMillis > playbackEndMillis) {
        qWarning("listening checkpoint is outside its playback interval");
        return {};
    }
    try {
        const auto state = session_->session_record_listening_progress(
            id.toStdString(),
            static_cast<std::uint64_t>(positionMillis),
            static_cast<std::uint64_t>(playbackStartMillis),
            static_cast<std::uint64_t>(playbackEndMillis)
        );
        QVariantMap result;
        result.insert(
            QStringLiteral("lastListenedAtMillis"),
            static_cast<qlonglong>(state.last_listened_at_millis)
        );
        result.insert(
            QStringLiteral("resumePositionMillis"),
            static_cast<qlonglong>(state.resume_position_millis)
        );
        emit listeningStateChanged(
            id,
            static_cast<qlonglong>(state.last_listened_at_millis),
            static_cast<qlonglong>(state.resume_position_millis)
        );
        return result;
    } catch (const rust::Error& error) {
        qWarning("cannot record listening progress for %s: %s", qPrintable(id), error.what());
        return {};
    }
}

bool DesktopBackend::setAssetAdjustment(
    const QString& id,
    qlonglong trimStartMillis,
    qlonglong trimEndMillis,
    qlonglong fadeInMillis,
    qlonglong fadeOutMillis,
    int fadeInCurve,
    int fadeOutCurve,
    int gainCentibels,
    int lowCutHertz,
    bool restorationEnabled,
    bool dePlosiveEnabled,
    int dePlosiveFrequencyHertz,
    int dePlosiveSensitivityPercent,
    int dePlosiveReductionCentibels,
    int dePlosiveReleaseMillis,
    bool noiseReductionEnabled,
    int noiseReductionCentibels,
    int noiseReductionSensitivityPercent,
    int noiseReductionSmoothingMillis,
    bool deEsserEnabled,
    int deEsserFrequencyHertz,
    int deEsserThresholdCentibels,
    int deEsserReductionCentibels,
    bool deHumEnabled,
    int deHumFundamentalHertz,
    int deHumHarmonicCount,
    int deHumQualityTenths,
    int deHumDepthCentibels,
    bool deClickEnabled,
    int deClickSensitivityPercent,
    int deClickMaximumClickMicroseconds,
    int deClickRepairPercent,
    bool channelRepairEnabled,
    bool channelRepairInvertLeft,
    bool channelRepairInvertRight,
    bool channelRepairSwapChannels,
    bool channelRepairMonoFoldDown,
    int channelRepairBalancePercent,
    bool equalizerEnabled,
    const QVariantList& equalizerBands,
    bool compressorEnabled,
    int compressorThresholdCentibels,
    int compressorRatioTenths,
    int compressorAttackMillis,
    int compressorReleaseMillis,
    int compressorMakeupCentibels,
    int reverbCharacter,
    bool reverbEnabled,
    int reverbMixPercent,
    int reverbPreDelayMillis,
    int reverbDecayMillis,
    int reverbSizePercent,
    int reverbDampingPercent,
    int reverbLowCutHertz,
    int reverbHighCutHertz,
    bool limiterEnabled,
    int limiterCeilingCentibels,
    int limiterReleaseMillis,
    const QVariantList& effectChain,
    const QVariantList& editSegments,
    const QVariantList& effectMasks
) {
    return setAssetAdjustment(
        id,
        trimStartMillis,
        trimEndMillis,
        fadeInMillis,
        fadeOutMillis,
        fadeInCurve,
        fadeOutCurve,
        gainCentibels,
        lowCutHertz,
        restorationEnabled,
        dePlosiveEnabled,
        dePlosiveFrequencyHertz,
        dePlosiveSensitivityPercent,
        dePlosiveReductionCentibels,
        dePlosiveReleaseMillis,
        noiseReductionEnabled,
        noiseReductionCentibels,
        noiseReductionSensitivityPercent,
        noiseReductionSmoothingMillis,
        deEsserEnabled,
        deEsserFrequencyHertz,
        deEsserThresholdCentibels,
        deEsserReductionCentibels,
        deHumEnabled,
        deHumFundamentalHertz,
        deHumHarmonicCount,
        deHumQualityTenths,
        deHumDepthCentibels,
        deClickEnabled,
        deClickSensitivityPercent,
        deClickMaximumClickMicroseconds,
        deClickRepairPercent,
        channelRepairEnabled,
        channelRepairInvertLeft,
        channelRepairInvertRight,
        channelRepairSwapChannels,
        channelRepairMonoFoldDown,
        channelRepairBalancePercent,
        equalizerEnabled,
        equalizerBands,
        compressorEnabled,
        compressorThresholdCentibels,
        compressorRatioTenths,
        compressorAttackMillis,
        compressorReleaseMillis,
        compressorMakeupCentibels,
        reverbCharacter,
        reverbEnabled,
        reverbMixPercent,
        reverbPreDelayMillis,
        reverbDecayMillis,
        reverbSizePercent,
        reverbDampingPercent,
        reverbLowCutHertz,
        reverbHighCutHertz,
        limiterEnabled,
        limiterCeilingCentibels,
        limiterReleaseMillis,
        effectChain,
        editSegments,
        effectMasks,
        {},
        {}
    );
}

bool DesktopBackend::setAssetAdjustment(
    const QString& id,
    qlonglong trimStartMillis,
    qlonglong trimEndMillis,
    qlonglong fadeInMillis,
    qlonglong fadeOutMillis,
    int fadeInCurve,
    int fadeOutCurve,
    int gainCentibels,
    int lowCutHertz,
    bool restorationEnabled,
    bool dePlosiveEnabled,
    int dePlosiveFrequencyHertz,
    int dePlosiveSensitivityPercent,
    int dePlosiveReductionCentibels,
    int dePlosiveReleaseMillis,
    bool noiseReductionEnabled,
    int noiseReductionCentibels,
    int noiseReductionSensitivityPercent,
    int noiseReductionSmoothingMillis,
    bool deEsserEnabled,
    int deEsserFrequencyHertz,
    int deEsserThresholdCentibels,
    int deEsserReductionCentibels,
    bool deHumEnabled,
    int deHumFundamentalHertz,
    int deHumHarmonicCount,
    int deHumQualityTenths,
    int deHumDepthCentibels,
    bool deClickEnabled,
    int deClickSensitivityPercent,
    int deClickMaximumClickMicroseconds,
    int deClickRepairPercent,
    bool channelRepairEnabled,
    bool channelRepairInvertLeft,
    bool channelRepairInvertRight,
    bool channelRepairSwapChannels,
    bool channelRepairMonoFoldDown,
    int channelRepairBalancePercent,
    bool equalizerEnabled,
    const QVariantList& equalizerBands,
    bool compressorEnabled,
    int compressorThresholdCentibels,
    int compressorRatioTenths,
    int compressorAttackMillis,
    int compressorReleaseMillis,
    int compressorMakeupCentibels,
    int reverbCharacter,
    bool reverbEnabled,
    int reverbMixPercent,
    int reverbPreDelayMillis,
    int reverbDecayMillis,
    int reverbSizePercent,
    int reverbDampingPercent,
    int reverbLowCutHertz,
    int reverbHighCutHertz,
    bool limiterEnabled,
    int limiterCeilingCentibels,
    int limiterReleaseMillis,
    const QVariantList& effectChain,
    const QVariantList& editSegments,
    const QVariantList& effectMasks,
    const QVariantMap& creativeVfx,
    const QVariantMap& space
) {
    const int space_mode = space.value(QStringLiteral("mode"), 0).toInt();
    const int convolution_mix = space.value(QStringLiteral("convolutionMixPercent"), 35).toInt();
    const int convolution_wet_gain =
        space.value(QStringLiteral("convolutionWetGainCentibels"), 0).toInt();
    const bool reverb_ducking_enabled =
        space.value(QStringLiteral("reverbDuckingEnabled"), false).toBool();
    const int reverb_ducking_amount =
        space.value(QStringLiteral("reverbDuckingAmountPercent"), 65).toInt();
    const int reverb_ducking_attack =
        space.value(QStringLiteral("reverbDuckingAttackMillis"), 10).toInt();
    const int reverb_ducking_release =
        space.value(QStringLiteral("reverbDuckingReleaseMillis"), 250).toInt();
    const QString impulse_import_id =
        space.value(QStringLiteral("impulseResponseImportId")).toString();
    const QString impulse_source_hash =
        space.value(QStringLiteral("impulseResponseSourceHash")).toString();
    const QString impulse_prepared_hash =
        space.value(QStringLiteral("impulseResponsePreparedHash")).toString();
    if (trimStartMillis < 0 || trimEndMillis < 0 || fadeInMillis < 0 || fadeOutMillis < 0
        || fadeInCurve < 0 || fadeInCurve > 2 || fadeOutCurve < 0 || fadeOutCurve > 2
        || gainCentibels < -2400 || gainCentibels > 1200
        || (lowCutHertz != 0 && (lowCutHertz < 20 || lowCutHertz > 240))
        || dePlosiveFrequencyHertz < 80 || dePlosiveFrequencyHertz > 240
        || dePlosiveSensitivityPercent < 0 || dePlosiveSensitivityPercent > 100
        || dePlosiveReductionCentibels < 0 || dePlosiveReductionCentibels > 1800
        || dePlosiveReleaseMillis < 40 || dePlosiveReleaseMillis > 500
        || noiseReductionCentibels < 0 || noiseReductionCentibels > 2400
        || noiseReductionSensitivityPercent < 0 || noiseReductionSensitivityPercent > 100
        || noiseReductionSmoothingMillis < 20 || noiseReductionSmoothingMillis > 1000
        || deEsserFrequencyHertz < 3000 || deEsserFrequencyHertz > 12000
        || deEsserThresholdCentibels < -6000 || deEsserThresholdCentibels > 0
        || deEsserReductionCentibels < 0 || deEsserReductionCentibels > 1800
        || (deHumFundamentalHertz != 50 && deHumFundamentalHertz != 60) || deHumHarmonicCount < 1
        || deHumHarmonicCount > 8 || deHumQualityTenths < 50 || deHumQualityTenths > 1000
        || deHumDepthCentibels < 0 || deHumDepthCentibels > 4800 || deClickSensitivityPercent < 0
        || deClickSensitivityPercent > 100 || deClickMaximumClickMicroseconds < 50
        || deClickMaximumClickMicroseconds > 2000 || deClickRepairPercent < 0
        || deClickRepairPercent > 100 || channelRepairBalancePercent < -100
        || channelRepairBalancePercent > 100 || compressorThresholdCentibels < -6000
        || compressorThresholdCentibels > 0 || compressorRatioTenths < 10
        || compressorRatioTenths > 200 || compressorAttackMillis < 1 || compressorAttackMillis > 200
        || compressorReleaseMillis < 20 || compressorReleaseMillis > 2000
        || compressorMakeupCentibels < 0 || compressorMakeupCentibels > 2400 || reverbCharacter < 0
        || reverbCharacter > 3 || limiterCeilingCentibels < -600 || limiterCeilingCentibels > 0
        || limiterReleaseMillis < 20 || limiterReleaseMillis > 1000 || reverbMixPercent < 0
        || reverbMixPercent > 100 || reverbPreDelayMillis < 0 || reverbPreDelayMillis > 200
        || reverbDecayMillis < 100 || reverbDecayMillis > 12000 || reverbSizePercent < 10
        || reverbSizePercent > 100 || reverbDampingPercent < 0 || reverbDampingPercent > 100
        || reverbLowCutHertz < 20 || reverbLowCutHertz > 1000 || reverbHighCutHertz < 1000
        || reverbHighCutHertz > 20000 || reverbLowCutHertz >= reverbHighCutHertz || space_mode < 0
        || space_mode > 1 || convolution_mix < 0 || convolution_mix > 100
        || convolution_wet_gain < -2400 || convolution_wet_gain > 1200 || reverb_ducking_amount < 0
        || reverb_ducking_amount > 100 || reverb_ducking_attack < 1 || reverb_ducking_attack > 200
        || reverb_ducking_release < 20 || reverb_ducking_release > 2000
        || (space_mode == 1
            && (impulse_import_id.isEmpty() || impulse_source_hash.isEmpty()
                || impulse_prepared_hash.isEmpty()))) {
        qWarning("sound adjustment is outside the supported range");
        return false;
    }
    try {
        const auto creative_vfx = CreativeVfxProjection::fromQml(creativeVfx);
        if (!creative_vfx.has_value()) {
            qWarning("creative VFX settings are outside the supported contract");
            return false;
        }
        echo::desktop::AssetAdjustmentWire adjustment;
        adjustment.trim_start_millis = static_cast<std::uint64_t>(trimStartMillis);
        adjustment.trim_end_millis = static_cast<std::uint64_t>(trimEndMillis);
        adjustment.fade_in_millis = static_cast<std::uint64_t>(fadeInMillis);
        adjustment.fade_out_millis = static_cast<std::uint64_t>(fadeOutMillis);
        adjustment.fade_in_curve = static_cast<std::uint8_t>(fadeInCurve);
        adjustment.fade_out_curve = static_cast<std::uint8_t>(fadeOutCurve);
        adjustment.gain_centibels = static_cast<std::int16_t>(gainCentibels);
        adjustment.low_cut_hertz = static_cast<std::uint16_t>(lowCutHertz);
        adjustment.restoration_enabled = restorationEnabled;
        adjustment.de_plosive_enabled = dePlosiveEnabled;
        adjustment.de_plosive_frequency_hertz = static_cast<std::uint16_t>(dePlosiveFrequencyHertz);
        adjustment.de_plosive_sensitivity_percent =
            static_cast<std::uint8_t>(dePlosiveSensitivityPercent);
        adjustment.de_plosive_reduction_centibels =
            static_cast<std::uint16_t>(dePlosiveReductionCentibels);
        adjustment.de_plosive_release_millis = static_cast<std::uint16_t>(dePlosiveReleaseMillis);
        adjustment.noise_reduction_enabled = noiseReductionEnabled;
        adjustment.noise_reduction_centibels = static_cast<std::uint16_t>(noiseReductionCentibels);
        adjustment.noise_reduction_sensitivity_percent =
            static_cast<std::uint8_t>(noiseReductionSensitivityPercent);
        adjustment.noise_reduction_smoothing_millis =
            static_cast<std::uint16_t>(noiseReductionSmoothingMillis);
        adjustment.de_esser_enabled = deEsserEnabled;
        adjustment.de_esser_frequency_hertz = static_cast<std::uint16_t>(deEsserFrequencyHertz);
        adjustment.de_esser_threshold_centibels =
            static_cast<std::int16_t>(deEsserThresholdCentibels);
        adjustment.de_esser_reduction_centibels =
            static_cast<std::uint16_t>(deEsserReductionCentibels);
        adjustment.de_hum_enabled = deHumEnabled;
        adjustment.de_hum_fundamental_hertz = static_cast<std::uint16_t>(deHumFundamentalHertz);
        adjustment.de_hum_harmonic_count = static_cast<std::uint8_t>(deHumHarmonicCount);
        adjustment.de_hum_quality_tenths = static_cast<std::uint16_t>(deHumQualityTenths);
        adjustment.de_hum_depth_centibels = static_cast<std::uint16_t>(deHumDepthCentibels);
        adjustment.de_click_enabled = deClickEnabled;
        adjustment.de_click_sensitivity_percent =
            static_cast<std::uint8_t>(deClickSensitivityPercent);
        adjustment.de_click_maximum_click_microseconds =
            static_cast<std::uint16_t>(deClickMaximumClickMicroseconds);
        adjustment.de_click_repair_percent = static_cast<std::uint8_t>(deClickRepairPercent);
        adjustment.channel_repair_enabled = channelRepairEnabled;
        adjustment.channel_repair_invert_left = channelRepairInvertLeft;
        adjustment.channel_repair_invert_right = channelRepairInvertRight;
        adjustment.channel_repair_swap_channels = channelRepairSwapChannels;
        adjustment.channel_repair_mono_fold_down = channelRepairMonoFoldDown;
        adjustment.channel_repair_balance_percent =
            static_cast<std::int8_t>(channelRepairBalancePercent);
        adjustment.equalizer_enabled = equalizerEnabled;
        if (!appendEqualizerBands(equalizerBands, adjustment.equalizer_bands)) {
            qWarning("parametric equalizer is outside the supported range");
            return false;
        }
        adjustment.compressor_enabled = compressorEnabled;
        adjustment.compressor_threshold_centibels =
            static_cast<std::int16_t>(compressorThresholdCentibels);
        adjustment.compressor_ratio_tenths = static_cast<std::uint16_t>(compressorRatioTenths);
        adjustment.compressor_attack_millis = static_cast<std::uint16_t>(compressorAttackMillis);
        adjustment.compressor_release_millis = static_cast<std::uint16_t>(compressorReleaseMillis);
        adjustment.compressor_makeup_centibels =
            static_cast<std::int16_t>(compressorMakeupCentibels);
        adjustment.reverb_character = static_cast<std::uint8_t>(reverbCharacter);
        adjustment.reverb_enabled = reverbEnabled;
        adjustment.reverb_mix_percent = static_cast<std::uint8_t>(reverbMixPercent);
        adjustment.reverb_pre_delay_millis = static_cast<std::uint16_t>(reverbPreDelayMillis);
        adjustment.reverb_decay_millis = static_cast<std::uint16_t>(reverbDecayMillis);
        adjustment.reverb_size_percent = static_cast<std::uint8_t>(reverbSizePercent);
        adjustment.reverb_damping_percent = static_cast<std::uint8_t>(reverbDampingPercent);
        adjustment.reverb_low_cut_hertz = static_cast<std::uint16_t>(reverbLowCutHertz);
        adjustment.reverb_high_cut_hertz = static_cast<std::uint16_t>(reverbHighCutHertz);
        adjustment.reverb_ducking_enabled = reverb_ducking_enabled;
        adjustment.reverb_ducking_amount_percent = static_cast<std::uint8_t>(reverb_ducking_amount);
        adjustment.reverb_ducking_attack_millis = static_cast<std::uint16_t>(reverb_ducking_attack);
        adjustment.reverb_ducking_release_millis =
            static_cast<std::uint16_t>(reverb_ducking_release);
        adjustment.space_mode = static_cast<std::uint8_t>(space_mode);
        adjustment.impulse_response_import_id = impulse_import_id.toStdString();
        adjustment.impulse_response_source_hash = impulse_source_hash.toStdString();
        adjustment.impulse_response_prepared_hash = impulse_prepared_hash.toStdString();
        adjustment.convolution_mix_percent = static_cast<std::uint8_t>(convolution_mix);
        adjustment.convolution_wet_gain_centibels = static_cast<std::int16_t>(convolution_wet_gain);
        adjustment.creative_vfx_json = CreativeVfxProjection::toJson(*creative_vfx).toStdString();
        adjustment.limiter_enabled = limiterEnabled;
        adjustment.limiter_ceiling_centibels = static_cast<std::int16_t>(limiterCeilingCentibels);
        adjustment.limiter_release_millis = static_cast<std::uint16_t>(limiterReleaseMillis);
        if (!appendEffectChain(effectChain, adjustment.effect_chain)) {
            qWarning("effect chain is outside the supported contract");
            return false;
        }
        if (!appendEditSegments(
                editSegments,
                trimStartMillis,
                trimEndMillis,
                adjustment.edit_segments
            )) {
            qWarning("source edit timeline is outside the supported contract");
            return false;
        }
        if (!appendEffectMasks(
                effectMasks,
                effectChain,
                trimStartMillis,
                trimEndMillis,
                adjustment.effect_masks
            )) {
            qWarning("effect masks are outside the supported contract");
            return false;
        }
        session_->session_set_asset_adjustment(id.toStdString(), adjustment);
        emit assetsChanged();
        return true;
    } catch (const rust::Error& error) {
        qWarning("cannot update adjustment for %s: %s", qPrintable(id), error.what());
        return false;
    }
}

QVariantList DesktopBackend::waveformForAsset(const QString& id) const {
    QVariantList levels;
    const auto artifact = session_->session_waveform_artifact(id.toStdString());
    for (const auto& level : artifact.levels) {
        QVariantList mins;
        QVariantList maxs;
        for (const float sample : level.mins) {
            mins.append(static_cast<double>(sample));
        }
        for (const float sample : level.maxs) {
            maxs.append(static_cast<double>(sample));
        }
        QVariantMap entry;
        entry.insert(
            QStringLiteral("samplesPerBucket"),
            static_cast<int>(level.samples_per_bucket)
        );
        entry.insert(QStringLiteral("mins"), mins);
        entry.insert(QStringLiteral("maxs"), maxs);
        levels.append(entry);
    }
    return levels;
}

QVariantList DesktopBackend::transcriptsForAsset(const QString& id) const {
    QVariantList transcripts;
    rust::Vec<echo::desktop::TranscriptWire> wires;
    try {
        wires = session_->session_transcripts(id.toStdString());
    } catch (const rust::Error& error) {
        qWarning("transcript query failed for %s: %s", qPrintable(id), error.what());
        return transcripts;
    }
    qInfo("transcripts for %s: %zu record(s)", qPrintable(id), wires.size());
    for (const auto& wire : wires) {
        QVariantList segments;
        for (const auto& segment : wire.segments) {
            QVariantMap entry;
            entry.insert(
                QStringLiteral("text"),
                QString::fromUtf8(segment.text.data(), segment.text.size())
            );
            entry.insert(QStringLiteral("start"), segment.start);
            entry.insert(QStringLiteral("end"), segment.end);
            segments.append(entry);
        }
        QVariantMap record;
        record.insert(
            QStringLiteral("model"),
            QString::fromUtf8(wire.model.data(), wire.model.size())
        );
        record.insert(
            QStringLiteral("modelVersion"),
            QString::fromUtf8(wire.model_version.data(), wire.model_version.size())
        );
        record.insert(
            QStringLiteral("language"),
            QString::fromUtf8(wire.language.data(), wire.language.size())
        );
        record.insert(
            QStringLiteral("text"),
            QString::fromUtf8(wire.text.data(), wire.text.size())
        );
        record.insert(QStringLiteral("segments"), segments);
        transcripts.append(record);
    }
    return transcripts;
}

QVariantList DesktopBackend::longAudioChaptersForAsset(const QString& id) const {
    QVariantList chapters;
    rust::Vec<echo::desktop::LongAudioChapterWire> wires;
    try {
        wires = session_->session_long_audio_chapters(id.toStdString());
    } catch (const rust::Error& error) {
        qWarning("long-audio outline query failed for %s: %s", qPrintable(id), error.what());
        return chapters;
    }
    for (const auto& wire : wires) {
        QVariantMap chapter;
        chapter.insert(QStringLiteral("level"), static_cast<quint32>(wire.level));
        chapter.insert(QStringLiteral("index"), static_cast<quint32>(wire.index));
        chapter.insert(QStringLiteral("startMillis"), static_cast<qulonglong>(wire.start_millis));
        chapter.insert(QStringLiteral("endMillis"), static_cast<qulonglong>(wire.end_millis));
        chapter.insert(
            QStringLiteral("soundCaption"),
            QString::fromUtf8(wire.sound_caption.data(), wire.sound_caption.size())
        );
        chapter.insert(
            QStringLiteral("summary"),
            QString::fromUtf8(wire.summary.data(), wire.summary.size())
        );
        chapters.append(chapter);
    }
    return chapters;
}

void DesktopBackend::startWorkers(const QString& runtimeEndpoint) {
    try {
        session_->session_start_workers(runtimeEndpoint.toStdString());
        workerStateRevision_ = session_->session_worker_state_revision();
        emit jobsChanged();
        analysisRefreshTimer_.start();
    } catch (const rust::Error& error) {
        qWarning("cannot start background workers: %s", error.what());
    }
}

QVariantMap DesktopBackend::analysisStatusForAsset(const QString& id) const {
    try {
        return analysisStatusForQml(session_->session_analysis_status(id.toStdString()));
    } catch (const rust::Error& error) {
        qWarning("cannot read analysis status for %s: %s", qPrintable(id), error.what());
    }
    return {};
}

QVariantList DesktopBackend::analysisStatuses() const {
    QVariantList statuses;
    try {
        const auto wires = session_->session_analysis_statuses();
        for (const auto& wire : wires) {
            statuses.append(analysisStatusForQml(wire));
        }
    } catch (const rust::Error& error) {
        qWarning("cannot read analysis statuses: %s", error.what());
    }
    return statuses;
}

bool DesktopBackend::retryAnalysis(const QString& id) {
    try {
        session_->session_retry_analysis(id.toStdString());
        emit jobsChanged();
        return true;
    } catch (const rust::Error& error) {
        qWarning("cannot retry analysis for %s: %s", qPrintable(id), error.what());
        return false;
    }
}

qulonglong DesktopBackend::retryFailedAnalysis() {
    try {
        const auto retried = session_->session_retry_failed_analysis();
        emit jobsChanged();
        return retried;
    } catch (const rust::Error& error) {
        qWarning("cannot retry failed analysis: %s", error.what());
        return 0;
    }
}

void DesktopBackend::queueScans() {
    try {
        const auto queued = session_->session_queue_scans();
        qInfo("queued %llu background scan(s)", queued);
        emit jobsChanged();
    } catch (const rust::Error& error) {
        qWarning("cannot queue scans: %s", error.what());
    }
}

QVariantMap DesktopBackend::jobStats() const {
    QVariantMap stats;
    try {
        const auto wire = session_->session_job_stats();
        stats.insert(QStringLiteral("pending"), static_cast<qlonglong>(wire.pending));
        stats.insert(QStringLiteral("running"), static_cast<qlonglong>(wire.running));
        stats.insert(QStringLiteral("done"), static_cast<qlonglong>(wire.done));
        stats.insert(QStringLiteral("failed"), static_cast<qlonglong>(wire.failed));
    } catch (const rust::Error& error) {
        qWarning("cannot read job stats: %s", error.what());
    }
    return stats;
}

QVariantList DesktopBackend::listRoots() const {
    QVariantList roots;
    try {
        const auto wires = session_->session_list_roots();
        for (const auto& wire : wires) {
            QVariantMap entry;
            entry.insert(QStringLiteral("id"), static_cast<qlonglong>(wire.id));
            entry.insert(
                QStringLiteral("root"),
                QString::fromUtf8(wire.root.data(), wire.root.size())
            );
            entry.insert(QStringLiteral("enabled"), wire.enabled);
            roots.append(entry);
        }
    } catch (const rust::Error& error) {
        qWarning("cannot list scan roots: %s", error.what());
    }
    return roots;
}

bool DesktopBackend::addRoot(const QUrl& folder) {
    if (!folder.isLocalFile()) {
        qWarning("cannot add non-local scan root %s", qPrintable(folder.toString()));
        return false;
    }
    const QString path = QDir::cleanPath(folder.toLocalFile());
    const QFileInfo info(path);
    if (!info.exists() || !info.isDir()) {
        qWarning("cannot add missing scan root %s", qPrintable(path));
        return false;
    }
    try {
        session_->session_add_root(path.toStdString());
        emit jobsChanged();
        return true;
    } catch (const rust::Error& error) {
        qWarning("cannot add scan root %s: %s", qPrintable(path), error.what());
        return false;
    }
}

void DesktopBackend::removeRoot(qlonglong id) {
    try {
        session_->session_remove_root(static_cast<std::int64_t>(id));
        emit jobsChanged();
    } catch (const rust::Error& error) {
        qWarning("cannot remove scan root: %s", error.what());
    }
}

QVariantList DesktopBackend::search(const QString& query) const {
    QVariantList results;
    if (query.trimmed().isEmpty()) {
        return results;
    }
    try {
        const auto wires = session_->session_search(query.toStdString(), 20);
        for (const auto& wire : wires) {
            QVariantMap entry;
            entry.insert(
                QStringLiteral("id"),
                QString::fromUtf8(wire.asset_id.data(), wire.asset_id.size())
            );
            entry.insert(
                QStringLiteral("path"),
                QString::fromUtf8(wire.path.data(), wire.path.size())
            );
            entry.insert(
                QStringLiteral("codec"),
                QString::fromUtf8(wire.codec.data(), wire.codec.size())
            );
            entry.insert(
                QStringLiteral("snippet"),
                QString::fromUtf8(wire.snippet.data(), wire.snippet.size())
            );
            entry.insert(QStringLiteral("startMillis"), static_cast<qlonglong>(wire.start_millis));
            results.append(entry);
        }
    } catch (const rust::Error& error) {
        qWarning("search failed: %s", error.what());
    }
    return results;
}

quint64 DesktopBackend::assetCount() const {
    return session_->session_asset_count();
}

QString DesktopBackend::catalogPath() const {
    const rust::String path = session_->session_catalog_path();
    return QString::fromUtf8(path.data(), path.size());
}

QString DesktopBackend::cacheRoot() const {
    const rust::String path = session_->session_cache_root();
    return QString::fromUtf8(path.data(), path.size());
}

QString DesktopBackend::recordRenderExport(
    const QString& assetId,
    qint64 adjustmentRevisionId,
    const QString& outputPath,
    const QString& format,
    quint32 sampleRate,
    quint32 channelCount,
    quint16 bitDepth,
    quint64 frameCount,
    quint64 sizeBytes,
    float integratedLufs,
    float truePeakDbtp
) const {
    try {
        echo::desktop::RenderExportWire evidence;
        evidence.output_path = outputPath.toStdString();
        evidence.format = format.toStdString();
        evidence.sample_rate = sampleRate;
        evidence.channel_count = channelCount;
        evidence.bit_depth = bitDepth;
        evidence.frame_count = frameCount;
        evidence.size_bytes = sizeBytes;
        evidence.integrated_lufs = integratedLufs;
        evidence.true_peak_dbtp = truePeakDbtp;
        session_
            ->session_record_render_export(assetId.toStdString(), adjustmentRevisionId, evidence);
        return {};
    } catch (const rust::Error& error) {
        return QString::fromUtf8(error.what());
    }
}
