"""The linked Cargo artifact, never an old cache, selects the generated ABI."""
from pathlib import Path
import importlib.util
import json
import subprocess
import sys
import tempfile

scripts = Path(__file__).resolve().parents[1] / "cmake"
spec = importlib.util.spec_from_file_location("build_bridge", scripts / "build_rust_bridge.py")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
with tempfile.TemporaryDirectory(prefix="echo-header-contract-") as directory:
    root = Path(directory)
    output = root / "include"
    receipt = root / "build.json"
    library = root / "libecho_desktop_bridge.a"
    library.write_bytes(b"library")
    current = root / "current/out"
    for name, content in (("current", "current ABI"), ("stale", "old ABI")):
        header = root / name / "out/cxxbridge/include/echo-desktop-bridge/src/lib.rs.h"
        header.parent.mkdir(parents=True)
        header.write_text(content)
    messages = [
        {"reason": "build-script-executed", "package_id": "desktop", "out_dir": str(current)},
        {"reason": "compiler-artifact", "package_id": "desktop", "target": {"name": "echo_desktop_bridge", "kind": ["staticlib"]}, "filenames": [str(library)], "fresh": True},
        {"reason": "build-finished", "success": True},
    ]
    def build(records, exit_code=0):
        command = [sys.executable, "-c", f"print({chr(10).join(map(json.dumps, records))!r}); raise SystemExit({exit_code})"]
        module.build(receipt, library, command)
    def rejects(records, exit_code=0):
        previous = receipt.read_bytes() if receipt.exists() else None
        try:
            build(records, exit_code)
        except RuntimeError:
            assert (receipt.read_bytes() if receipt.exists() else None) == previous
        else:
            raise AssertionError("invalid build accepted")
    rejects([])
    build(messages)
    result = subprocess.run(["cmake", f"-DECHO_CARGO_BUILD_RECEIPT={receipt}",
        f"-DECHO_CXXBRIDGE_INCLUDE_DIRECTORY={output}", "-P", str(scripts / "sync_cxxbridge_headers.cmake")], capture_output=True, text=True)
    assert result.returncode == 0, result.stderr
    assert (output / "echo-desktop-bridge/src/lib.rs.h").read_text() == "current ABI"
    rejects(messages, 1)
    rejects(messages[:-1])
    rejects([messages[0], messages[-1]])
    rejects([{"reason": "build-script-executed", "package_id": "desktop", "out_dir": str(root / "stale/out")}] + messages)
    (current / "cxxbridge/include/echo-desktop-bridge/src/lib.rs.h").unlink()
    rejects(messages)
print("cxxbridge artifact identity contract: passed")
