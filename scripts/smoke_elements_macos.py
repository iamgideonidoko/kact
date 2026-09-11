#!/usr/bin/env python3
"""Check native AX element discovery against Kact's stable fixture window."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import time

if sys.platform != "darwin":
    raise SystemExit("This check requires a macOS desktop and Accessibility permission.")
binary = Path(sys.argv[1] if len(sys.argv) > 1 else "target/debug/kact").resolve()
source = Path(__file__).resolve().parent / "fixtures" / "elements.swift"

with tempfile.TemporaryDirectory(prefix="kact-elements-") as directory:
    directory = Path(directory)
    fixture_binary = directory / "fixture"
    subprocess.run(["swiftc", str(source), "-o", str(fixture_binary)], check=True)
    fixture = subprocess.Popen([str(fixture_binary)], stdout=subprocess.PIPE, text=True)
    try:
        deadline = time.monotonic() + 8
        ready = None
        while time.monotonic() < deadline:
            line = fixture.stdout.readline()
            if line:
                event = json.loads(line)
                if event.get("ready"):
                    ready = event
                    break
            assert fixture.poll() is None, "fixture exited before becoming ready"
        assert ready, "fixture never became ready"
        result = subprocess.run(
            [str(binary), "inspect", "elements", "--pid", str(ready["pid"])],
            capture_output=True,
            text=True,
            timeout=8,
            check=True,
        )
        report = json.loads(result.stdout)
        assert report["mode"] == "accessibility", report
        assert report["node_count"] >= report["target_count"] >= 4, report
        targets = report["targets"]
        assert {target["role"] for target in targets} >= {"AXButton", "AXCheckBox", "AXTextField", "AXSlider"}, targets
        assert all(target["text"] is None for target in targets), targets
        assert all(target["bounds"]["width"] > 1 and target["bounds"]["height"] > 1 for target in targets), targets
    finally:
        if fixture.poll() is None:
            fixture.terminate()
            try:
                fixture.wait(timeout=5)
            except subprocess.TimeoutExpired:
                fixture.kill()
                fixture.wait()

print("Passed: native accessibility fixture structural snapshot.")
