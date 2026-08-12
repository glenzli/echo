#pragma once

// FFmpeg 8.x public headers no longer carry `extern "C"` guards. C++ engine
// sources must include FFmpeg through this wrapper so every FFmpeg symbol
// keeps C linkage; nothing else may include the raw FFmpeg headers.

extern "C" {
#include <libavcodec/avcodec.h>
#include <libavformat/avformat.h>
#include <libavutil/channel_layout.h>
#include <libavutil/error.h>
#include <libavutil/opt.h>
#include <libavutil/parseutils.h>
#include <libavutil/rational.h>
#include <libavutil/samplefmt.h>
#include <libswresample/swresample.h>
}
