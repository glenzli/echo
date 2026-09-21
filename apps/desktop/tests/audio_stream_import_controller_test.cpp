#include "audio_stream_import_controller.hpp"
#include "echo/audio/decode.hpp"
#include "echo/audio/playback.hpp"
#include <QCoreApplication>
#include <QCryptographicHash>
#include <QDir>
#include <QElapsedTimer>
#include <QFile>
#include <QFileInfo>
#include <QTemporaryDir>
#include <QThread>
#include <cassert>
#include <cmath>
#include <iostream>
template <class F> void until(F ready) {
    QElapsedTimer clock;
    clock.start();
    while (!ready()) {
        assert(clock.elapsed() < 15000);
        QCoreApplication::processEvents();
        QThread::msleep(1);
    }
}
QByteArray digest(const QString& path) {
    QFile file(path);
    assert(file.open(QIODevice::ReadOnly));
    QCryptographicHash hash(QCryptographicHash::Sha256);
    assert(hash.addData(&file));
    return hash.result().toHex();
}
int main(int argc, char** argv) {
    QCoreApplication app(argc, argv);
    QTemporaryDir root;
    assert(root.isValid());
    AudioStreamImportController importer(root.path());
    importer.inspect({});
    assert(!importer.errorText().isEmpty());
    importer.inspect({QUrl("https://invalid.example/audio")});
    assert(!importer.errorText().isEmpty());
    if (argc < 2)
        return 0;
    const auto source = QUrl::fromLocalFile(QString::fromUtf8(argv[1]));
    const auto before = digest(source.toLocalFile());
    QList<QUrl> result;
    QObject::connect(&importer, &AudioStreamImportController::filesReady, [&](const auto& files) {
        result = files;
    });
    importer.inspect({source});
    until([&] { return !importer.busy(); });
    assert(importer.choosing() && importer.streams().size() == 2);
    importer.select(999);
    assert(importer.choosing() && !importer.busy());
    assert(importer.streams()[1].toMap().value("durationMillis").toULongLong() == 2000);
    const int second = importer.streams()[1].toMap().value("index").toInt();
    importer.select(second);
    until([&] { return !result.isEmpty() || !importer.errorText().isEmpty(); });
    if (!importer.errorText().isEmpty())
        std::cerr << importer.errorText().toStdString() << '\n';
    assert(importer.errorText().isEmpty());
    assert(result.size() == 1);
    const auto selected = result[0];
    const auto preserved = QFileInfo(selected.toLocalFile()).dir().filePath("original");
    assert(digest(preserved) == before && digest(source.toLocalFile()) == before);
    const auto metadata = echo::audio::probe(selected.toLocalFile().toStdString());
    bool streamMatches = false, hashMatches = false, labels = false;
    for (const auto& entry : metadata.metadata) {
        const auto key = QString::fromStdString(entry.key).toLower();
        if (key == "echo_source_sha256")
            hashMatches = QString::fromStdString(entry.value).toLatin1() == before;
        if (key == "echo_source_stream_index")
            streamMatches = entry.value == std::to_string(second);
        if (entry.value.find("Echo source disclosure:") == 0)
            labels = true;
    }
    assert(hashMatches && streamMatches && labels);
    assert(metadata.duration_millis == 2000);
    echo::audio::PlaybackSession player(
        selected.toLocalFile().toStdString(),
        {},
        {.apply_output_guard = false, .collect_metering = false}
    );
    std::vector<float> pcm(8192);
    double first = 0, secondEnergy = 0;
    std::uint64_t frames = 0;
    until([&] {
        const auto count = player.read(pcm.data(), 4096);
        for (std::size_t i = 0; i < count; ++i) {
            const double t = static_cast<double>(frames + i) / 48000.0;
            first += pcm[2 * i] * std::sin(2 * 3.141592653589793 * 440 * t);
            secondEnergy += pcm[2 * i] * std::sin(2 * 3.141592653589793 * 880 * t);
        }
        frames += count;
        return player.is_ended() && !player.buffered_frames();
    });
    assert(std::abs(secondEnergy) > 10 * std::abs(first));
    result.clear();
    importer.inspect({source});
    until([&] { return importer.choosing(); });
    importer.select(second);
    until([&] { return !result.isEmpty(); });
    assert(result[0] == selected);
    result.clear();
    importer.inspect({selected});
    until([&] { return !result.isEmpty(); });
    assert(result[0] == selected);
    importer.inspect({source});
    until([&] { return importer.choosing(); });
    importer.cancel();
    assert(!importer.choosing() && !importer.busy());
    for (int i = 2; i < argc; ++i) {
        result.clear();
        importer.inspect({QUrl::fromLocalFile(QString::fromUtf8(argv[i]))});
        until([&] { return !importer.busy(); });
        assert(!importer.errorText().isEmpty() && result.isEmpty());
        importer.inspect({selected});
        until([&] { return !result.isEmpty(); });
        assert(importer.errorText().isEmpty());
    }
    std::cout << "Selected second track, preserved original, labels and identity; reuse, "
                 "single-track passthrough and cancel passed\n";
}
