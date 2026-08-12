#include "creative_vfx_projection.hpp"

#include <QJsonDocument>
#include <QJsonObject>
#include <QStringList>

namespace {

QVariant
field(const QVariantMap& value, const char* camel, const char* snake, const QVariant& fallback) {
    const QString camel_key = QString::fromLatin1(camel);
    if (value.contains(camel_key)) {
        return value.value(camel_key);
    }
    return value.value(QString::fromLatin1(snake), fallback);
}

std::optional<int> character(const QVariant& value, const QStringList& names) {
    bool integer_ok = false;
    const int integer = value.toInt(&integer_ok);
    if (integer_ok && integer >= 0 && integer < names.size()) {
        return integer;
    }
    const qsizetype named = names.indexOf(value.toString());
    return named >= 0 ? std::optional<int>(static_cast<int>(named)) : std::nullopt;
}

QVariantMap nested(const QVariantMap& value, const char* name) {
    return value.value(QString::fromLatin1(name)).toMap();
}

QVariantMap familyHeader(int kind, bool enabled) {
    return {
        {QStringLiteral("character"), kind},
        {QStringLiteral("enabled"), enabled},
    };
}

} // namespace

std::optional<echo::audio::CreativeVfxAdjustment>
CreativeVfxProjection::fromQml(const QVariantMap& value) {
    const QVariantMap scene = nested(value, "scene");
    const auto scene_character = character(
        scene.value(QStringLiteral("character"), 0),
        {"telephone", "radio", "intercom", "behind_wall", "underwater"}
    );
    const int scene_mix = field(scene, "mixPercent", "mix_percent", 100).toInt();
    const int scene_intensity = field(scene, "intensityPercent", "intensity_percent", 50).toInt();

    const QVariantMap delay = nested(value, "delay");
    const auto delay_character =
        character(delay.value(QStringLiteral("character"), 0), {"slapback", "echo"});
    const QVariantMap slapback = nested(delay, "slapback");
    const int slapback_time = field(slapback, "delayMillis", "delay_millis", 90).toInt();
    const int slapback_mix = field(slapback, "mixPercent", "mix_percent", 22).toInt();
    const int slapback_cut = field(slapback, "highCutHertz", "high_cut_hertz", 7000).toInt();
    const QVariantMap echo = nested(delay, "echo");
    const int echo_time = field(echo, "delayMillis", "delay_millis", 375).toInt();
    const int echo_feedback = field(echo, "feedbackPercent", "feedback_percent", 36).toInt();
    const int echo_mix = field(echo, "mixPercent", "mix_percent", 28).toInt();
    const int echo_cut = field(echo, "highCutHertz", "high_cut_hertz", 6500).toInt();
    const int echo_cross =
        field(echo, "stereoCrossfeedPercent", "stereo_crossfeed_percent", 70).toInt();

    const QVariantMap modulation = nested(value, "modulation");
    const auto modulation_character = character(
        modulation.value(QStringLiteral("character"), 0),
        {"chorus", "flanger", "phaser", "tremolo"}
    );
    const QVariantMap chorus = nested(modulation, "chorus");
    const int chorus_mix = field(chorus, "mixPercent", "mix_percent", 35).toInt();
    const int chorus_rate = field(chorus, "rateMillihertz", "rate_millihertz", 800).toInt();
    const int chorus_minimum =
        field(chorus, "minimumDelayMicroseconds", "minimum_delay_microseconds", 8000).toInt();
    const int chorus_sweep =
        field(chorus, "sweepMicroseconds", "sweep_microseconds", 10000).toInt();
    const int chorus_phase =
        field(chorus, "stereoPhaseDegrees", "stereo_phase_degrees", 90).toInt();
    const QVariantMap flanger = nested(modulation, "flanger");
    const int flanger_mix = field(flanger, "mixPercent", "mix_percent", 50).toInt();
    const int flanger_rate = field(flanger, "rateMillihertz", "rate_millihertz", 250).toInt();
    const int flanger_minimum =
        field(flanger, "minimumDelayMicroseconds", "minimum_delay_microseconds", 200).toInt();
    const int flanger_sweep =
        field(flanger, "sweepMicroseconds", "sweep_microseconds", 3500).toInt();
    const int flanger_feedback = field(flanger, "feedbackPercent", "feedback_percent", 35).toInt();
    const int flanger_phase =
        field(flanger, "stereoPhaseDegrees", "stereo_phase_degrees", 180).toInt();
    const QVariantMap phaser = nested(modulation, "phaser");
    const int phaser_mix = field(phaser, "mixPercent", "mix_percent", 50).toInt();
    const int phaser_rate = field(phaser, "rateMillihertz", "rate_millihertz", 350).toInt();
    const int phaser_low = field(phaser, "sweepLowHertz", "sweep_low_hertz", 300).toInt();
    const int phaser_high = field(phaser, "sweepHighHertz", "sweep_high_hertz", 2500).toInt();
    const int phaser_feedback = field(phaser, "feedbackPercent", "feedback_percent", 25).toInt();
    const int phaser_phase =
        field(phaser, "stereoPhaseDegrees", "stereo_phase_degrees", 90).toInt();
    const QVariantMap tremolo = nested(modulation, "tremolo");
    const int tremolo_rate = field(tremolo, "rateMillihertz", "rate_millihertz", 4000).toInt();
    const int tremolo_depth = field(tremolo, "depthPercent", "depth_percent", 60).toInt();
    const int tremolo_phase =
        field(tremolo, "stereoPhaseDegrees", "stereo_phase_degrees", 0).toInt();

    const QVariantMap transform = nested(value, "transform");
    const auto transform_character = character(
        transform.value(QStringLiteral("character"), 0),
        {"robot", "monster", "tiny", "giant", "ghost"}
    );
    const int transform_mix = field(transform, "mixPercent", "mix_percent", 100).toInt();
    const int transform_amount = field(transform, "amountPercent", "amount_percent", 50).toInt();

    const QVariantMap digital_degrade = nested(value, "digital_degrade").isEmpty()
                                            ? nested(value, "digitalDegrade")
                                            : nested(value, "digital_degrade");
    const auto digital_degrade_character = character(
        digital_degrade.value(QStringLiteral("character"), 0),
        {"bitcrusher", "sample_rate_reduction", "lo_fi"}
    );
    const int digital_mix = field(digital_degrade, "mixPercent", "mix_percent", 100).toInt();
    const QVariantMap bitcrusher = nested(digital_degrade, "bitcrusher");
    const int bit_depth = field(bitcrusher, "bitDepth", "bit_depth", 8).toInt();
    const QVariantMap sample_rate_reduction =
        nested(digital_degrade, "sample_rate_reduction").isEmpty()
            ? nested(digital_degrade, "sampleRateReduction")
            : nested(digital_degrade, "sample_rate_reduction");
    const int target_rate =
        field(sample_rate_reduction, "targetRateHertz", "target_rate_hertz", 8000).toInt();

    if (!scene_character || scene_mix < 0 || scene_mix > 100 || scene_intensity < 0
        || scene_intensity > 100 || !delay_character || slapback_time < 30 || slapback_time > 180
        || slapback_mix < 0 || slapback_mix > 100 || slapback_cut < 1000 || slapback_cut > 20000
        || echo_time < 80 || echo_time > 2000 || echo_feedback < 0 || echo_feedback > 90
        || echo_mix < 0 || echo_mix > 100 || echo_cut < 1000 || echo_cut > 20000 || echo_cross < 0
        || echo_cross > 100 || !modulation_character || chorus_mix < 0 || chorus_mix > 100
        || chorus_rate < 50 || chorus_rate > 5000 || chorus_minimum < 5000 || chorus_minimum > 25000
        || chorus_sweep < 500 || chorus_sweep > 20000 || chorus_minimum + chorus_sweep > 45000
        || chorus_phase < 0 || chorus_phase > 180 || flanger_mix < 0 || flanger_mix > 100
        || flanger_rate < 50 || flanger_rate > 10000 || flanger_minimum < 100
        || flanger_minimum > 5000 || flanger_sweep < 100 || flanger_sweep > 10000
        || flanger_minimum + flanger_sweep > 15000 || flanger_feedback < -90
        || flanger_feedback > 90 || flanger_phase < 0 || flanger_phase > 180 || phaser_mix < 0
        || phaser_mix > 100 || phaser_rate < 50 || phaser_rate > 10000 || phaser_low < 20
        || phaser_high > 20000 || phaser_low >= phaser_high || phaser_feedback < -90
        || phaser_feedback > 90 || phaser_phase < 0 || phaser_phase > 180 || tremolo_rate < 100
        || tremolo_rate > 20000 || tremolo_depth < 0 || tremolo_depth > 100 || tremolo_phase < 0
        || tremolo_phase > 180 || !transform_character || transform_mix < 0 || transform_mix > 100
        || transform_amount < 0 || transform_amount > 100 || !digital_degrade_character
        || digital_mix < 0 || digital_mix > 100 || bit_depth < 2 || bit_depth > 16
        || target_rate < 1000 || target_rate > 24000) {
        return std::nullopt;
    }

    return echo::audio::CreativeVfxAdjustment{
        .scene =
            {
                .character = static_cast<echo::audio::SceneVfxCharacter>(*scene_character),
                .enabled = scene.value(QStringLiteral("enabled"), false).toBool(),
                .mix_percent = static_cast<std::uint8_t>(scene_mix),
                .intensity_percent = static_cast<std::uint8_t>(scene_intensity),
            },
        .delay =
            {
                .character = static_cast<echo::audio::DelayVfxCharacter>(*delay_character),
                .enabled = delay.value(QStringLiteral("enabled"), false).toBool(),
                .slapback =
                    {
                        .delay_millis = static_cast<std::uint16_t>(slapback_time),
                        .mix_percent = static_cast<std::uint8_t>(slapback_mix),
                        .high_cut_hertz = static_cast<std::uint16_t>(slapback_cut),
                    },
                .echo =
                    {
                        .delay_millis = static_cast<std::uint16_t>(echo_time),
                        .feedback_percent = static_cast<std::uint8_t>(echo_feedback),
                        .mix_percent = static_cast<std::uint8_t>(echo_mix),
                        .high_cut_hertz = static_cast<std::uint16_t>(echo_cut),
                        .stereo_crossfeed_percent = static_cast<std::uint8_t>(echo_cross),
                    },
            },
        .modulation =
            {
                .character =
                    static_cast<echo::audio::ModulationVfxCharacter>(*modulation_character),
                .enabled = modulation.value(QStringLiteral("enabled"), false).toBool(),
                .chorus =
                    {
                        .mix_percent = static_cast<std::uint8_t>(chorus_mix),
                        .rate_millihertz = static_cast<std::uint16_t>(chorus_rate),
                        .minimum_delay_microseconds = static_cast<std::uint16_t>(chorus_minimum),
                        .sweep_microseconds = static_cast<std::uint16_t>(chorus_sweep),
                        .stereo_phase_degrees = static_cast<std::uint16_t>(chorus_phase),
                    },
                .flanger =
                    {
                        .mix_percent = static_cast<std::uint8_t>(flanger_mix),
                        .rate_millihertz = static_cast<std::uint16_t>(flanger_rate),
                        .minimum_delay_microseconds = static_cast<std::uint16_t>(flanger_minimum),
                        .sweep_microseconds = static_cast<std::uint16_t>(flanger_sweep),
                        .feedback_percent = static_cast<std::int8_t>(flanger_feedback),
                        .stereo_phase_degrees = static_cast<std::uint16_t>(flanger_phase),
                    },
                .phaser =
                    {
                        .mix_percent = static_cast<std::uint8_t>(phaser_mix),
                        .rate_millihertz = static_cast<std::uint16_t>(phaser_rate),
                        .sweep_low_hertz = static_cast<std::uint16_t>(phaser_low),
                        .sweep_high_hertz = static_cast<std::uint16_t>(phaser_high),
                        .feedback_percent = static_cast<std::int8_t>(phaser_feedback),
                        .stereo_phase_degrees = static_cast<std::uint16_t>(phaser_phase),
                    },
                .tremolo =
                    {
                        .rate_millihertz = static_cast<std::uint16_t>(tremolo_rate),
                        .depth_percent = static_cast<std::uint8_t>(tremolo_depth),
                        .stereo_phase_degrees = static_cast<std::uint16_t>(tremolo_phase),
                    },
            },
        .transform =
            {
                .character = static_cast<echo::audio::TransformVfxCharacter>(*transform_character),
                .enabled = transform.value(QStringLiteral("enabled"), false).toBool(),
                .mix_percent = static_cast<std::uint8_t>(transform_mix),
                .amount_percent = static_cast<std::uint8_t>(transform_amount),
            },
        .digital_degrade = {
            .character =
                static_cast<echo::audio::DigitalDegradeVfxCharacter>(*digital_degrade_character),
            .enabled = digital_degrade.value(QStringLiteral("enabled"), false).toBool(),
            .mix_percent = static_cast<std::uint8_t>(digital_mix),
            .bitcrusher = {.bit_depth = static_cast<std::uint8_t>(bit_depth)},
            .sample_rate_reduction = {
                .target_rate_hertz = static_cast<std::uint16_t>(target_rate),
            },
        },
    };
}

std::optional<echo::audio::CreativeVfxAdjustment>
CreativeVfxProjection::fromJson(const QByteArray& encoded) {
    const QJsonDocument document = QJsonDocument::fromJson(encoded);
    return document.isObject() ? fromQml(document.object().toVariantMap()) : std::nullopt;
}

QVariantMap CreativeVfxProjection::toQml(const echo::audio::CreativeVfxAdjustment& adjustment) {
    QVariantMap scene =
        familyHeader(static_cast<int>(adjustment.scene.character), adjustment.scene.enabled);
    scene.insert(QStringLiteral("mixPercent"), adjustment.scene.mix_percent);
    scene.insert(QStringLiteral("intensityPercent"), adjustment.scene.intensity_percent);

    QVariantMap delay =
        familyHeader(static_cast<int>(adjustment.delay.character), adjustment.delay.enabled);
    delay.insert(
        QStringLiteral("slapback"),
        QVariantMap{
            {QStringLiteral("delayMillis"), adjustment.delay.slapback.delay_millis},
            {QStringLiteral("mixPercent"), adjustment.delay.slapback.mix_percent},
            {QStringLiteral("highCutHertz"), adjustment.delay.slapback.high_cut_hertz},
        }
    );
    delay.insert(
        QStringLiteral("echo"),
        QVariantMap{
            {QStringLiteral("delayMillis"), adjustment.delay.echo.delay_millis},
            {QStringLiteral("feedbackPercent"), adjustment.delay.echo.feedback_percent},
            {QStringLiteral("mixPercent"), adjustment.delay.echo.mix_percent},
            {QStringLiteral("highCutHertz"), adjustment.delay.echo.high_cut_hertz},
            {QStringLiteral("stereoCrossfeedPercent"),
             adjustment.delay.echo.stereo_crossfeed_percent},
        }
    );

    QVariantMap modulation = familyHeader(
        static_cast<int>(adjustment.modulation.character),
        adjustment.modulation.enabled
    );
    modulation.insert(
        QStringLiteral("chorus"),
        QVariantMap{
            {QStringLiteral("mixPercent"), adjustment.modulation.chorus.mix_percent},
            {QStringLiteral("rateMillihertz"), adjustment.modulation.chorus.rate_millihertz},
            {QStringLiteral("minimumDelayMicroseconds"),
             adjustment.modulation.chorus.minimum_delay_microseconds},
            {QStringLiteral("sweepMicroseconds"), adjustment.modulation.chorus.sweep_microseconds},
            {QStringLiteral("stereoPhaseDegrees"),
             adjustment.modulation.chorus.stereo_phase_degrees},
        }
    );
    modulation.insert(
        QStringLiteral("flanger"),
        QVariantMap{
            {QStringLiteral("mixPercent"), adjustment.modulation.flanger.mix_percent},
            {QStringLiteral("rateMillihertz"), adjustment.modulation.flanger.rate_millihertz},
            {QStringLiteral("minimumDelayMicroseconds"),
             adjustment.modulation.flanger.minimum_delay_microseconds},
            {QStringLiteral("sweepMicroseconds"), adjustment.modulation.flanger.sweep_microseconds},
            {QStringLiteral("feedbackPercent"), adjustment.modulation.flanger.feedback_percent},
            {QStringLiteral("stereoPhaseDegrees"),
             adjustment.modulation.flanger.stereo_phase_degrees},
        }
    );
    modulation.insert(
        QStringLiteral("phaser"),
        QVariantMap{
            {QStringLiteral("mixPercent"), adjustment.modulation.phaser.mix_percent},
            {QStringLiteral("rateMillihertz"), adjustment.modulation.phaser.rate_millihertz},
            {QStringLiteral("sweepLowHertz"), adjustment.modulation.phaser.sweep_low_hertz},
            {QStringLiteral("sweepHighHertz"), adjustment.modulation.phaser.sweep_high_hertz},
            {QStringLiteral("feedbackPercent"), adjustment.modulation.phaser.feedback_percent},
            {QStringLiteral("stereoPhaseDegrees"),
             adjustment.modulation.phaser.stereo_phase_degrees},
        }
    );
    modulation.insert(
        QStringLiteral("tremolo"),
        QVariantMap{
            {QStringLiteral("rateMillihertz"), adjustment.modulation.tremolo.rate_millihertz},
            {QStringLiteral("depthPercent"), adjustment.modulation.tremolo.depth_percent},
            {QStringLiteral("stereoPhaseDegrees"),
             adjustment.modulation.tremolo.stereo_phase_degrees},
        }
    );

    QVariantMap transform = familyHeader(
        static_cast<int>(adjustment.transform.character),
        adjustment.transform.enabled
    );
    transform.insert(QStringLiteral("mixPercent"), adjustment.transform.mix_percent);
    transform.insert(QStringLiteral("amountPercent"), adjustment.transform.amount_percent);
    QVariantMap digital_degrade = familyHeader(
        static_cast<int>(adjustment.digital_degrade.character),
        adjustment.digital_degrade.enabled
    );
    digital_degrade.insert(QStringLiteral("mixPercent"), adjustment.digital_degrade.mix_percent);
    digital_degrade.insert(
        QStringLiteral("bitcrusher"),
        QVariantMap{{QStringLiteral("bitDepth"), adjustment.digital_degrade.bitcrusher.bit_depth}}
    );
    digital_degrade.insert(
        QStringLiteral("sampleRateReduction"),
        QVariantMap{
            {QStringLiteral("targetRateHertz"),
             adjustment.digital_degrade.sample_rate_reduction.target_rate_hertz}
        }
    );
    return {
        {QStringLiteral("scene"), scene},
        {QStringLiteral("delay"), delay},
        {QStringLiteral("modulation"), modulation},
        {QStringLiteral("transform"), transform},
        {QStringLiteral("digitalDegrade"), digital_degrade},
    };
}

QByteArray CreativeVfxProjection::toJson(const echo::audio::CreativeVfxAdjustment& adjustment) {
    const QVariantMap qml = toQml(adjustment);
    auto enum_name = [](int value, const QStringList& names) { return names.at(value); };
    QVariantMap scene = nested(qml, "scene");
    scene[QStringLiteral("character")] = enum_name(
        scene.value(QStringLiteral("character")).toInt(),
        {"telephone", "radio", "intercom", "behind_wall", "underwater"}
    );
    QVariantMap delay = nested(qml, "delay");
    delay[QStringLiteral("character")] =
        enum_name(delay.value(QStringLiteral("character")).toInt(), {"slapback", "echo"});
    QVariantMap modulation = nested(qml, "modulation");
    modulation[QStringLiteral("character")] = enum_name(
        modulation.value(QStringLiteral("character")).toInt(),
        {"chorus", "flanger", "phaser", "tremolo"}
    );
    QVariantMap transform = nested(qml, "transform");
    transform[QStringLiteral("character")] = enum_name(
        transform.value(QStringLiteral("character")).toInt(),
        {"robot", "monster", "tiny", "giant", "ghost"}
    );
    QVariantMap digital_degrade = nested(qml, "digitalDegrade");
    digital_degrade[QStringLiteral("character")] = enum_name(
        digital_degrade.value(QStringLiteral("character")).toInt(),
        {"bitcrusher", "sample_rate_reduction", "lo_fi"}
    );
    auto snake = [](QVariantMap map, const QString& camel, const QString& snake_name) {
        map.insert(snake_name, map.take(camel));
        return map;
    };
    scene = snake(scene, "mixPercent", "mix_percent");
    scene = snake(scene, "intensityPercent", "intensity_percent");
    auto delay_params = [&](QVariantMap map) {
        map = snake(map, "delayMillis", "delay_millis");
        map = snake(map, "mixPercent", "mix_percent");
        map = snake(map, "highCutHertz", "high_cut_hertz");
        return map;
    };
    QVariantMap slapback = delay_params(nested(delay, "slapback"));
    QVariantMap echo = delay_params(nested(delay, "echo"));
    echo = snake(echo, "feedbackPercent", "feedback_percent");
    echo = snake(echo, "stereoCrossfeedPercent", "stereo_crossfeed_percent");
    delay[QStringLiteral("slapback")] = slapback;
    delay[QStringLiteral("echo")] = echo;
    auto common_modulation = [&](QVariantMap map) {
        map = snake(map, "rateMillihertz", "rate_millihertz");
        map = snake(map, "stereoPhaseDegrees", "stereo_phase_degrees");
        return map;
    };
    QVariantMap chorus = common_modulation(nested(modulation, "chorus"));
    chorus = snake(chorus, "mixPercent", "mix_percent");
    chorus = snake(chorus, "minimumDelayMicroseconds", "minimum_delay_microseconds");
    chorus = snake(chorus, "sweepMicroseconds", "sweep_microseconds");
    QVariantMap flanger = common_modulation(nested(modulation, "flanger"));
    flanger = snake(flanger, "mixPercent", "mix_percent");
    flanger = snake(flanger, "minimumDelayMicroseconds", "minimum_delay_microseconds");
    flanger = snake(flanger, "sweepMicroseconds", "sweep_microseconds");
    flanger = snake(flanger, "feedbackPercent", "feedback_percent");
    QVariantMap phaser = common_modulation(nested(modulation, "phaser"));
    phaser = snake(phaser, "mixPercent", "mix_percent");
    phaser = snake(phaser, "sweepLowHertz", "sweep_low_hertz");
    phaser = snake(phaser, "sweepHighHertz", "sweep_high_hertz");
    phaser = snake(phaser, "feedbackPercent", "feedback_percent");
    QVariantMap tremolo = common_modulation(nested(modulation, "tremolo"));
    tremolo = snake(tremolo, "depthPercent", "depth_percent");
    modulation[QStringLiteral("chorus")] = chorus;
    modulation[QStringLiteral("flanger")] = flanger;
    modulation[QStringLiteral("phaser")] = phaser;
    modulation[QStringLiteral("tremolo")] = tremolo;
    transform = snake(transform, "mixPercent", "mix_percent");
    transform = snake(transform, "amountPercent", "amount_percent");
    digital_degrade = snake(digital_degrade, "mixPercent", "mix_percent");
    QVariantMap digital_bitcrusher = nested(digital_degrade, "bitcrusher");
    digital_bitcrusher = snake(digital_bitcrusher, "bitDepth", "bit_depth");
    QVariantMap digital_rate = nested(digital_degrade, "sampleRateReduction");
    digital_rate = snake(digital_rate, "targetRateHertz", "target_rate_hertz");
    digital_degrade[QStringLiteral("bitcrusher")] = digital_bitcrusher;
    digital_degrade.remove(QStringLiteral("sampleRateReduction"));
    digital_degrade[QStringLiteral("sample_rate_reduction")] = digital_rate;
    return QJsonDocument::fromVariant(
               QVariantMap{
                   {QStringLiteral("scene"), scene},
                   {QStringLiteral("delay"), delay},
                   {QStringLiteral("modulation"), modulation},
                   {QStringLiteral("transform"), transform},
                   {QStringLiteral("digital_degrade"), digital_degrade},
               }
    )
        .toJson(QJsonDocument::Compact);
}
