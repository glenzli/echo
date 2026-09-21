"""Run Cargo once and bind generated headers to the static library it built."""
import json
import os
from pathlib import Path
import subprocess
import sys


def build(receipt, library, command):
    receipt = Path(receipt)
    library = Path(library).resolve()
    outputs = {}
    packages = set()
    succeeded = False
    # Preserve Make's inherited jobserver descriptors, as the former direct
    # Cargo invocation did. The CMake target declares JOB_SERVER_AWARE.
    process = subprocess.Popen(command, stdout=subprocess.PIPE, text=True, close_fds=False)
    for line in process.stdout:
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            print(line, end="")
            continue
        if not isinstance(message, dict):
            print(line, end="")
            continue
        reason = message.get("reason")
        if reason == "compiler-message":
            rendered = message.get("message", {}).get("rendered")
            if rendered:
                print(rendered, end="", file=sys.stderr)
        elif reason == "build-script-executed":
            outputs.setdefault(message["package_id"], set()).add(message["out_dir"])
        elif reason == "compiler-artifact":
            target = message.get("target", {})
            if target.get("name") == "echo_desktop_bridge" and "staticlib" in target.get("kind", []):
                if library in [Path(p).resolve() for p in message.get("filenames", [])]:
                    packages.add(message["package_id"])
        elif reason == "build-finished":
            succeeded = message.get("success") is True
    code = process.wait()
    if code or not succeeded:
        raise RuntimeError(f"Cargo did not finish successfully ({code})")
    if len(packages) != 1 or not library.is_file():
        raise RuntimeError("Cargo did not report exactly the expected desktop static library")
    directories = outputs.get(packages.pop(), set())
    if len(directories) != 1:
        raise RuntimeError("Cargo did not report one desktop bridge output directory")
    include = Path(directories.pop()) / "cxxbridge/include"
    if not (include / "echo-desktop-bridge/src/lib.rs.h").is_file():
        raise RuntimeError("Cargo's desktop bridge output is missing its generated header")
    receipt.parent.mkdir(parents=True, exist_ok=True)
    temporary = receipt.with_suffix(".pending")
    temporary.write_text(json.dumps({"include_directory": str(include.resolve())}) + "\n")
    os.replace(temporary, receipt)


if __name__ == "__main__":
    try:
        build(sys.argv[1], sys.argv[2], sys.argv[3:])
    except (OSError, RuntimeError) as error:
        sys.exit(f"desktop Rust bridge: {error}")
