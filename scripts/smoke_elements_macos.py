#!/usr/bin/env python3
"""Check bounded, visible native AX discovery against a changing fixture window."""
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
        def inspect(show_text=False):
            command = [str(binary), "inspect", "elements", "--pid", str(ready["pid"])]
            if show_text:
                command.append("--show-text")
            result = subprocess.run(
                command,
                capture_output=True,
                text=True,
                timeout=8,
                check=True,
            )
            return json.loads(result.stdout)

        report = inspect()
        assert report["mode"] == "accessibility", report
        assert report["node_count"] >= report["target_count"] >= 4, report
        # A 120-item document must stay bounded. Exact counts vary by macOS AX
        # implementation; the invariant is that visible discovery never labels
        # every offscreen row as an actionable target.
        assert report["target_count"] < 40, report
        assert report["duration_ms"] < 1_000, report
        targets = report["targets"]
        assert {target["role"] for target in targets} >= {"AXButton", "AXCheckBox", "AXTextField", "AXSlider"}, targets
        assert all(target["text"] is None for target in targets), targets
        assert all(target["bounds"]["width"] > 1 and target["bounds"]["height"] > 1 for target in targets), targets

        text_report = inspect(show_text=True)
        def text_value(target):
            value = target["text"]
            return " ".join(value) if isinstance(value, list) else value

        visible_text = {value for target in text_report["targets"] for value in target["text"]}
        assert "Document item 119" in visible_text, text_report
        assert "Document item 0" not in visible_text, text_report
        assert "Nested visible action" in visible_text, text_report
        assert "Nested offscreen action" not in visible_text, text_report

        deadline = time.monotonic() + 5
        layout = None
        while time.monotonic() < deadline:
            line = fixture.stdout.readline()
            if line:
                event = json.loads(line)
                if event.get("event") == "layout":
                    layout = event
                    break
            assert fixture.poll() is None, "fixture exited before relayout"
        assert layout, "fixture never relaid out"
        moved = inspect(show_text=True)
        moved_target = next((target for target in moved["targets"] if "Primary action moved" in target["text"]), None)
        assert moved_target, moved
        assert moved_target["bounds"]["x"] > 150, moved
    finally:
        if fixture.poll() is None:
            fixture.terminate()
            try:
                fixture.wait(timeout=5)
            except subprocess.TimeoutExpired:
                fixture.kill()
                fixture.wait()

print("Passed: visible, bounded native accessibility fixture snapshot and relayout.")
