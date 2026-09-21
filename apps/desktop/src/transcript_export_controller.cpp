#include "transcript_export_controller.hpp"
#include <QClipboard>
#include <QFileInfo>
#include <QGuiApplication>
#include <QSaveFile>
#include <cmath>
#include <limits>

namespace {
struct Cue {
    qint64 start, end;
    QString text;
};
QList<Cue> cues(const QVariantMap& record) {
    QList<Cue> result;
    qint64 previous = -1;
    for (const auto& value : record.value("segments").toList()) {
        const auto segment = value.toMap();
        bool startOk = false, endOk = false;
        const double start = segment.value("start").toDouble(&startOk);
        const double end = segment.value("end").toDouble(&endOk);
        auto text = segment.value("text").toString().trimmed();
        if (!startOk || !endOk || !std::isfinite(start) || !std::isfinite(end) || start < 0
            || end <= start || end >= static_cast<double>(std::numeric_limits<qint64>::max() / 1000)
            || text.isEmpty())
            return {};
        const auto first = static_cast<qint64>(std::llround(start * 1000));
        const auto last = static_cast<qint64>(std::llround(end * 1000));
        if (last <= first || first < previous)
            return {};
        // Subtitle formats interpret markup and blank lines; keep recognized text literal.
        text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
        text.replace('\r', ' ').replace('\n', ' ');
        result.push_back({first, last, text});
        previous = first;
    }
    return result;
}
QString timestamp(qint64 millis, QChar separator) {
    return QString("%1:%2:%3%4%5")
        .arg(millis / 3600000, 2, 10, QLatin1Char('0'))
        .arg((millis / 60000) % 60, 2, 10, QLatin1Char('0'))
        .arg((millis / 1000) % 60, 2, 10, QLatin1Char('0'))
        .arg(separator)
        .arg(millis % 1000, 3, 10, QLatin1Char('0'));
}
} // namespace
bool TranscriptExportController::hasTiming(const QVariantMap& record) const {
    return !cues(record).isEmpty();
}
QString TranscriptExportController::copyText(const QVariantMap& record) const {
    const auto text = record.value("text").toString();
    if (text.trimmed().isEmpty())
        return tr("There is no transcript text to copy.");
    QGuiApplication::clipboard()->setText(text);
    return {};
}
QString TranscriptExportController::save(
    const QVariantMap& record,
    const QUrl& destination,
    const QString& format,
    const QString& sourcePath
) const {
    if (!destination.isLocalFile() || (format != "txt" && format != "srt" && format != "vtt"))
        return tr("Choose a local TXT, SRT, or VTT file.");
    const QFileInfo target(destination.toLocalFile());
    if (target.suffix().toLower() != format)
        return tr("The file extension must match the selected transcript format.");
    const QFileInfo source(sourcePath);
    if (!sourcePath.isEmpty()
        && (target.absoluteFilePath() == source.absoluteFilePath()
            || (!source.canonicalFilePath().isEmpty()
                && target.canonicalFilePath() == source.canonicalFilePath())))
        return tr("Choose a different file to preserve the original recording.");
    QString output;
    if (format == "txt") {
        output = record.value("text").toString();
        if (output.trimmed().isEmpty())
            return tr("There is no transcript text to export.");
        if (!output.endsWith('\n'))
            output += '\n';
    } else {
        const auto timed = cues(record);
        if (timed.isEmpty())
            return tr("Subtitle export needs valid sentence timing. Export plain text instead.");
        if (format == "vtt")
            output = "WEBVTT\n\n";
        const QChar separator = format == "srt" ? ',' : '.';
        int index = 0;
        for (const auto& cue : timed) {
            output += QString::number(++index) + '\n' + timestamp(cue.start, separator) + " --> "
                      + timestamp(cue.end, separator) + '\n' + cue.text + "\n\n";
        }
    }
    QSaveFile file(target.absoluteFilePath());
    const auto bytes = output.toUtf8();
    if (!file.open(QIODevice::WriteOnly) || file.write(bytes) != bytes.size() || !file.commit())
        return tr("The transcript could not be saved. Check the destination and try again.");
    return {};
}
