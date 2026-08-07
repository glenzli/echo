#!/usr/bin/env python3
"""Echo ASR worker: runs mlx_audio transcription as a subprocess contract.

Echo's Rust side invokes this script with the MLX venv's python. The script
loads the resolved model snapshot and writes a normalized transcript JSON
with the schema owned by echo-core:

    {
      "model": "mlx/qwen3-asr",
      "language": "zh",
      "text": "...",
      "segments": [
        {"text": "...", "start": 0.0, "end": 5.46}
      ]
    }
"""

import argparse
import json
import sys
import tempfile
import os


def main() -> int:
    parser = argparse.ArgumentParser(description="Echo ASR worker")
    parser.add_argument("--model", required=True, help="resolved model snapshot path")
    parser.add_argument("--audio", required=True, help="audio file to transcribe")
    parser.add_argument("--out", required=True, help="output JSON path")
    args = parser.parse_args()

    from mlx_audio.stt.generate import generate_transcription

    with tempfile.NamedTemporaryFile(suffix=".json", delete=False) as handle:
        raw_path = handle.name
    try:
        generate_transcription(
            args.model,
            args.audio,
            output_path=os.path.splitext(raw_path)[0],
            format="json",
        )
        with open(raw_path, encoding="utf-8") as handle:
            raw = json.load(handle)
    finally:
        if os.path.exists(raw_path):
            os.remove(raw_path)
        for candidate in (os.path.splitext(raw_path)[0] + ".json",):
            if os.path.exists(candidate):
                os.remove(candidate)

    def normalize_units(units):
        if not units:
            return []
        return [
            {
                "text": unit.get("text", ""),
                "start": float(unit.get("start", 0.0)),
                "end": float(unit.get("end", 0.0)),
            }
            for unit in units
        ]

    # mlx_audio emits either `sentences` (word tokens nested) or `segments`.
    if "sentences" in raw:
        segments = [
            {
                "text": sentence.get("text", ""),
                "start": float(sentence.get("start", 0.0)),
                "end": float(sentence.get("end", 0.0)),
                "words": normalize_units(sentence.get("tokens")),
            }
            for sentence in raw["sentences"]
        ]
    else:
        segments = [
            {
                "text": segment.get("text", ""),
                "start": float(segment.get("start", 0.0)),
                "end": float(segment.get("end", 0.0)),
            }
            for segment in raw.get("segments", [])
        ]

    payload = {
        "model": "mlx/qwen3-asr",
        "language": raw.get("language"),
        "text": raw.get("text", ""),
        "segments": segments,
    }
    with open(args.out, "w", encoding="utf-8") as handle:
        json.dump(payload, handle, ensure_ascii=False, indent=2)
    return 0


if __name__ == "__main__":
    sys.exit(main())
