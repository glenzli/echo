#!/usr/bin/env python3
"""Generate bounded, deterministic codec fixtures with local FFmpeg (no downloads).

Usage: prepare_audio_format_fixtures.py /absolute/output-directory
Run echo-audio-export-test on the files listed in manifest.json's playback list.
APE and AMR remain intake candidates, not covered by this local encoder matrix.
"""
import json
from pathlib import Path
import shutil
import subprocess
import sys


def main():
    root = Path(sys.argv[1]).resolve()
    root.mkdir(parents=True, exist_ok=True)
    ffmpeg = shutil.which("ffmpeg")
    if not ffmpeg:
        raise SystemExit("A local FFmpeg executable is required")
    specs = {
        "pcm16.wav": ["-c:a", "pcm_s16le"],
        "pcm24.wav": ["-c:a", "pcm_s24le"],
        "float32.wav": ["-c:a", "pcm_f32le"],
        "large-container.rf64": ["-c:a", "pcm_s24le", "-rf64", "always", "-f", "wav"],
        "wave64.w64": ["-c:a", "pcm_s24le"],
        "lossless.flac": ["-c:a", "flac"],
        "variable.mp3": ["-c:a", "libmp3lame", "-q:a", "3"],
        "aac.m4a": ["-c:a", "aac"],
        "alac.m4a": ["-c:a", "alac"],
        "book.m4b": ["-c:a", "aac", "-f", "ipod"],
        "raw.aac": ["-c:a", "aac"],
        "pcm.aiff": ["-c:a", "pcm_s24be"],
        "float.aifc": ["-c:a", "pcm_f32be", "-f", "aiff"],
        "coreaudio.caf": ["-c:a", "pcm_s24le"],
        "vorbis.ogg": ["-c:a", "vorbis", "-strict", "-2"],
        "speech.opus": ["-c:a", "libopus"],
        "windows.wma": ["-c:a", "wmav2"],
        "wavpack.wv": ["-c:a", "wavpack"],
        "web.webm": ["-c:a", "libopus"],
        "matroska.mka": ["-c:a", "flac"],
        "movie.mp4": ["-c:a", "aac"],
        "quicktime.mov": ["-c:a", "pcm_s24le"],
        "mobile.3gp": ["-c:a", "aac", "-ar", "44100"],
    }
    for name, options in specs.items():
        subprocess.run([ffmpeg, "-v", "error", "-y", "-f", "lavfi", "-i",
                        "sine=frequency=440:sample_rate=48000:duration=2", "-ac", "2",
                        *options, str(root / name)], check=True)
    shutil.copyfile(root / "pcm24.wav", root / "UPPER.WAV")
    comment = 'Echo source disclosure: {"schema":"echo.source-disclosure.v1","scope":"referenced_sources","kinds":["ai_generated"]}'
    subprocess.run([ffmpeg, "-v", "error", "-y", "-f", "lavfi", "-i",
                    "sine=frequency=440:duration=2", "-f", "lavfi", "-i",
                    "sine=frequency=880:duration=2", "-map", "0:a", "-map", "1:a",
                    "-c:a", "flac", "-metadata:s:a:0", "language=eng",
                    "-metadata:s:a:1", "language=zho", "-metadata", f"comment={comment}",
                    str(root / "two-tracks.mka")], check=True)
    subprocess.run([ffmpeg, "-v", "error", "-y", "-f", "lavfi", "-i",
                    "sine=frequency=440:duration=5", "-itsoffset", "2", "-f", "lavfi", "-i",
                    "sine=frequency=880:duration=2", "-map", "0:a", "-map", "1:a",
                    "-c:a", "flac", "-metadata", f"comment={comment}",
                    str(root / "offset-tracks.mka")], check=True)
    (root / "malformed.wav").write_text("not an audio file\n")
    subprocess.run([ffmpeg, "-v", "error", "-y", "-f", "lavfi", "-i",
                    "color=size=32x32:duration=1", "-an", "-c:v", "mpeg4",
                    str(root / "silent-video.mp4")], check=True)
    manifest = {"playback": [*specs, "UPPER.WAV"], "multipleTracks": ["two-tracks.mka", "offset-tracks.mka"],
                "rejected": ["malformed.wav", "silent-video.mp4"],
                "notCovered": ["APE", "AMR", "physical export larger than 4 GiB"]}
    (root / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"Prepared {len(manifest['playback'])} playback variants and stream/error fixtures")


if __name__ == "__main__":
    main()
