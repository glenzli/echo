#!/usr/bin/env python3
"""Temporary direct MLX execution for Echo's Infer-compatible audio request.

The JSON-lines envelope intentionally matches Infer Runtime's audio worker
transport. This process performs no routing, admission, retry, or lifecycle
management; Infer Build will replace it without changing Echo's intent shape.
"""

from __future__ import annotations

import contextlib
import json
import os
import subprocess
import sys
import tempfile
import traceback
from pathlib import Path
from typing import Any

os.environ.setdefault("HF_HUB_OFFLINE", "1")
os.environ.setdefault("TRANSFORMERS_OFFLINE", "1")


def field(value: Any, name: str, default: Any = None) -> Any:
    if isinstance(value, dict):
        return value.get(name, default)
    return getattr(value, name, default)


def scalar(value: Any) -> Any:
    if isinstance(value, (list, tuple)):
        return value[0] if value else None
    return value


def normalize_units(units: Any) -> list[dict[str, Any]]:
    if not units:
        return []
    return [
        {
            "text": str(field(unit, "text", "")),
            "start": float(field(unit, "start", field(unit, "start_time", 0.0))),
            "end": float(field(unit, "end", field(unit, "end_time", 0.0))),
        }
        for unit in units
    ]


@contextlib.contextmanager
def normalized_audio_source(audio_path: str):
    """Bridges formats the temporary MLX decoder cannot read directly."""
    if Path(audio_path).suffix.lower() not in {".aif", ".aiff"}:
        yield audio_path
        return

    with tempfile.TemporaryDirectory(prefix="echo-asr-") as directory:
        normalized_path = Path(directory) / "source.wav"
        try:
            subprocess.run(
                [
                    "ffmpeg",
                    "-nostdin",
                    "-loglevel",
                    "error",
                    "-y",
                    "-i",
                    audio_path,
                    "-ac",
                    "1",
                    "-ar",
                    "16000",
                    str(normalized_path),
                ],
                check=True,
                capture_output=True,
                text=True,
            )
        except FileNotFoundError as error:
            raise RuntimeError(
                "AIFF transcription requires ffmpeg in the local adapter"
            ) from error
        except subprocess.CalledProcessError as error:
            detail = error.stderr.strip() or "ffmpeg conversion failed"
            raise RuntimeError(detail) from error
        yield str(normalized_path)


def transcribe(request: dict[str, Any]) -> dict[str, Any]:
    intent = request.get("intent") or {}
    if intent.get("model") != "audio.transcribe":
        raise ValueError("direct adapter only accepts audio.transcribe")

    with contextlib.redirect_stdout(sys.stderr):
        from mlx_audio.stt.utils import load_model

        model = load_model(request["model"])
        kwargs: dict[str, Any] = {"verbose": False}
        if intent.get("language"):
            kwargs["language"] = intent["language"]
        if intent.get("prompt"):
            kwargs["system_prompt"] = intent["prompt"]
        if intent.get("temperature") is not None:
            kwargs["temperature"] = intent["temperature"]
        with normalized_audio_source(request["audio_path"]) as audio_source:
            result = model.generate(audio_source, **kwargs)

    raw_segments = field(result, "segments", field(result, "sentences", [])) or []
    segments = []
    for segment in raw_segments:
        normalized = {
            "text": str(field(segment, "text", "")),
            "start": float(
                field(segment, "start", field(segment, "start_time", 0.0))
            ),
            "end": float(field(segment, "end", field(segment, "end_time", 0.0))),
        }
        words = normalize_units(field(segment, "words", field(segment, "tokens")))
        if words:
            normalized["words"] = words
        segments.append(normalized)

    return {
        "model": "mlx/qwen3-asr",
        "language": scalar(field(result, "language")),
        "text": str(scalar(field(result, "text", "")) or ""),
        "segments": segments,
    }


def handle(request: dict[str, Any]) -> dict[str, Any]:
    if request.get("operation") == "transcribe":
        return transcribe(request)
    raise ValueError(f"unsupported operation: {request.get('operation')}")


def main() -> int:
    for line in sys.stdin:
        if not line.strip():
            continue
        request_id = "unknown"
        try:
            request = json.loads(line)
            request_id = str(request.get("request_id", request_id))
            result = handle(request)
            response = {"request_id": request_id, "ok": True, "result": result}
        except Exception as error:
            traceback.print_exc(file=sys.stderr)
            response = {"request_id": request_id, "ok": False, "error": str(error)}
        sys.stdout.write(json.dumps(response, ensure_ascii=False) + "\n")
        sys.stdout.flush()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
