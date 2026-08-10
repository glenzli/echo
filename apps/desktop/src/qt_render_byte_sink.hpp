#pragma once

#include <QFileDevice>

#include <span>
#include <stdexcept>

#include "echo/audio/offline_render.hpp"

class QtRenderByteSink final : public echo::audio::RenderByteSink {
  public:
    explicit QtRenderByteSink(QFileDevice& file) : file_(file) {}

    void write(std::span<const std::byte> bytes) override {
        const qint64 size = static_cast<qint64>(bytes.size());
        const qint64 written = file_.write(reinterpret_cast<const char*>(bytes.data()), size);
        if (written != size) {
            throw std::runtime_error(file_.errorString().toStdString());
        }
    }

    void seek(std::uint64_t offset) override {
        if (!file_.seek(static_cast<qint64>(offset))) {
            throw std::runtime_error(file_.errorString().toStdString());
        }
    }

  private:
    QFileDevice& file_;
};
