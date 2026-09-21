#include "transcript_export_controller.hpp"
#include <QClipboard>
#include <QFile>
#include <QGuiApplication>
#include <QTemporaryDir>
#include <cassert>
#include <limits>

static QByteArray read(const QString& path) {
    QFile file(path); assert(file.open(QIODevice::ReadOnly)); return file.readAll();
}
int main(int argc, char** argv) {
    QGuiApplication app(argc, argv);
    TranscriptExportController exporter;
    QTemporaryDir root; assert(root.isValid());
    QVariantMap record{{"text", QString::fromUtf8("雨声与脚步\nSecond sentence.")}, {"segments", QVariantList{
        QVariantMap{{"start", 61.125}, {"end", 63.456}, {"text", "雨声 <literal> & steps"}},
        QVariantMap{{"start", 3601.0}, {"end", 3602.0}, {"text", "Two\n\nlines"}}}}};
    assert(exporter.hasTiming(record));
    assert(exporter.copyText(record).isEmpty());
    assert(QGuiApplication::clipboard()->text() == record["text"]);
    for (const auto& format : {QString("txt"), QString("srt"), QString("vtt")}) {
        const auto path = root.filePath("transcript." + format);
        assert(exporter.save(record, QUrl::fromLocalFile(path), format, "").isEmpty());
        const auto bytes = read(path);
        assert(bytes.contains("雨声"));
        if (format == "txt") assert(bytes == record["text"].toString().toUtf8() + '\n');
        else {
            assert(bytes.contains("&lt;literal&gt; &amp; steps"));
            assert(bytes.contains("Two  lines"));
            assert(bytes.contains(format == "srt" ? "00:01:01,125 --> 00:01:03,456" : "00:01:01.125 --> 00:01:03.456"));
            assert(bytes.startsWith(format == "vtt" ? "WEBVTT\n\n1\n" : "1\n"));
            assert(bytes.contains("01:00:01"));
        }
    }
    const auto path = root.filePath("transcript.srt");
    const auto original = read(path);
    for (const auto& segment : QVariantList{
             QVariantMap{{"start", -1}, {"end", 1}, {"text", "bad"}},
             QVariantMap{{"start", 1}, {"end", 1.0001}, {"text", "bad"}},
             QVariantMap{{"start", 2}, {"end", 1}, {"text", "bad"}},
             QVariantMap{{"start", 0}, {"end", std::numeric_limits<double>::infinity()}, {"text", "bad"}},
             QVariantMap{{"start", 0}, {"end", 1e100}, {"text", "bad"}},
             QVariantMap{{"end", 1}, {"text", "missing start"}}}) {
        auto invalid = record; invalid["segments"] = QVariantList{segment};
        assert(!exporter.hasTiming(invalid));
        assert(!exporter.save(invalid, QUrl::fromLocalFile(path), "srt", "").isEmpty());
        assert(read(path) == original);
    }
    auto untimed = record; untimed["segments"] = QVariantList{};
    assert(!exporter.hasTiming(untimed));
    assert(exporter.save(untimed, QUrl::fromLocalFile(root.filePath("text.txt")), "txt", "").isEmpty());
    assert(!exporter.save(record, QUrl::fromLocalFile(path), "srt", path).isEmpty());
    const auto link = root.filePath("alias.srt"); assert(QFile::link(path, link));
    assert(!exporter.save(record, QUrl::fromLocalFile(link), "srt", path).isEmpty());
    assert(!exporter.save(record, QUrl::fromLocalFile(path), "txt", "").isEmpty());
    assert(!exporter.save(record, QUrl("https://example.com/text.txt"), "txt", "").isEmpty());
    assert(!exporter.save(record, QUrl::fromLocalFile(root.filePath("absent/text.txt")), "txt", "").isEmpty());
    assert(read(path) == original);
}
