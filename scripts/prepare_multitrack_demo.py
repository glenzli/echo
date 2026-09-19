#!/usr/bin/env python3
"""Fetch attributed CC0 sounds and request a local, explicitly synthetic voice fixture.

This is a developer fixture tool, not Echo's consumer identity or a product
generation feature. Pass the existing local operator credential explicitly.
Audio and request provenance stay in the chosen directory, outside Git.
"""

import argparse
import hashlib
import json
from pathlib import Path
import urllib.request
import wave


SOURCES = [
    {
        "file": "Rain in the gutter.mp3",
        "title": "Rain in the Gutter Loop",
        "author": "Ogrebane",
        "kind": "recorded_ambience",
        "license": "CC0-1.0",
        "page": "https://lpc.opengameart.org/content/rain-gutter-loop",
        "url": "https://lpc.opengameart.org/sites/default/files/rain-gutter-loop_0.mp3",
        "sha256": "216f57eebace492b1d56226c13a728582735de7282c5cca43059cd3c960f083e",
    },
    {
        "file": "Forest ambience music.mp3",
        "title": "Forest Ambience",
        "author": "TinyWorlds",
        "kind": "music",
        "license": "CC0-1.0",
        "page": "https://opengameart.org/content/forest-ambience",
        "url": "https://opengameart.org/sites/default/files/Forest_Ambience.mp3",
        "sha256": "9850aa1d0d5d66bd9c5daf8bb77c6d852e01f2f4de22f283bd5621e8bed13b75",
    },
]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("destination", type=Path)
    parser.add_argument("--operator-credential", required=True, type=Path)
    parser.add_argument("--runtime-port", type=int, default=8787)
    args = parser.parse_args()
    args.destination.mkdir(parents=True, exist_ok=True)
    manifest_path = args.destination / "sources.json"
    previous = json.loads(manifest_path.read_text()) if manifest_path.exists() else {"sources": []}
    output = args.destination / "After the rain — synthetic narration.wav"
    generated = next((source for source in previous["sources"] if source.get("kind") == "synthetic_speech"), None)
    if output.exists() and (not generated or hashlib.sha256(output.read_bytes()).hexdigest() != generated["sha256"]):
        raise RuntimeError("Existing narration has no matching provenance; use a new destination")
    manifest = {"purpose": "Echo multitrack debugging; synthetic voice is not a personal recording", "sources": []}

    def record(entry):
        path = args.destination / entry["file"]
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        if entry.get("sha256", digest) != digest:
            raise RuntimeError("Fixture hash changed: " + entry["file"])
        entry["sha256"] = digest
        entry["bytes"] = path.stat().st_size
        manifest["sources"].append(entry)
        manifest_path.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")
        print(json.dumps({"file": entry["file"], "bytes": entry["bytes"], "kind": entry["kind"]}), flush=True)

    for source in SOURCES:
        destination = args.destination / source["file"]
        if not destination.exists():
            request = urllib.request.Request(source["url"], headers={"User-Agent": "Echo-Debug-Fixtures/1.0"})
            with urllib.request.urlopen(request, timeout=60) as response:
                if "audio" not in response.headers.get("Content-Type", ""):
                    raise RuntimeError("Expected audio from the attributed download")
                data = response.read(32 * 1024 * 1024 + 1)
                if len(data) > 32 * 1024 * 1024:
                    raise RuntimeError("Fixture download exceeded the size limit")
                destination.write_bytes(data)
        record(dict(source))

    if generated and output.exists():
        record(generated)
        return

    prompt = "雨停以后，我们沿着河边慢慢走。树叶还在滴水，远处偶尔传来脚步声。把这一刻留下来，以后再听。"
    payload = {
        "model": "speech.design_voice", "input": prompt,
        "instructions": "温暖、自然的成年女声，用普通话轻声讲述，节奏舒缓，发音清晰。不要模仿任何真实人物。",
        "language": "Chinese", "response_format": "wav",
        "metadata": {"infer.placement": "local_only", "infer.offline_required": "true"},
    }
    headers = {
        "Authorization": "Bearer " + args.operator_credential.read_text().strip(),
        "Content-Type": "application/json",
        "Infer-Consumer-Contract": "infer-runtime.consumer-core@20260813.1",
        "Infer-Capability-Contract": "infer.audio.speech@20260811.1",
    }
    request = urllib.request.Request(
        f"http://127.0.0.1:{args.runtime_port}/v1/audio/speech",
        data=json.dumps(payload).encode(), headers=headers, method="POST",
    )
    print("Requesting local synthetic narration…", flush=True)
    with urllib.request.urlopen(request, timeout=600) as response:
        data = response.read()
        provenance = {key: value for key, value in response.headers.items() if key.lower().startswith("x-infer-")}
    output.write_bytes(data)
    with wave.open(str(output)) as audio:
        duration = audio.getnframes() / audio.getframerate()
        if duration <= 1:
            raise RuntimeError("Generated narration is too short for the fixture")
    entry = {"file": output.name, "kind": "synthetic_speech", "provider": "Infer Runtime", "request": payload,
             "response": provenance, "duration_seconds": duration}
    # Persist the successful response before the optional diagnostic lookup.
    record(entry)
    job_id = next((value for key, value in provenance.items() if key.lower() == "x-infer-job-id"), None)
    if job_id:
        request = urllib.request.Request(f"http://127.0.0.1:{args.runtime_port}/infer/v1/jobs/{job_id}", headers=headers)
        with urllib.request.urlopen(request, timeout=30) as response:
            job = json.load(response)
        entry["job"] = {key: job.get(key) for key in ("id", "deployment", "model_build", "model_profile", "physical_model", "provider", "placement", "attempts", "error")}
        manifest_path.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")


if __name__ == "__main__":
    main()
