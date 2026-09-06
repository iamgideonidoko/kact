#!/usr/bin/env python3
"""Test real mouse events inside a temporary native window; restore the cursor afterward."""
import ctypes
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import time

if sys.platform != "darwin":
    raise SystemExit("This check requires macOS and Accessibility permission.")
binary = Path(sys.argv[1] if len(sys.argv) > 1 else "target/debug/kact").resolve()
fixture_source = Path(__file__).resolve().parent / "fixtures" / "mouse.swift"
with tempfile.TemporaryDirectory(prefix="kact-mouse-") as directory:
    directory = Path(directory)
    fixture_binary = directory / "receiver"
    subprocess.run(["swiftc", str(fixture_source), "-o", str(fixture_binary)], check=True)
    config = directory / "config.toml"
    config.write_text("[keybindings]\nnavigation_enabled = false\n")
    args = [str(binary), "--config", str(config), "--socket", str(directory / "private/control.sock")]
    with (directory / "daemon.log").open("w") as daemon_log, (directory / "events.jsonl").open("w") as event_log:
        daemon = subprocess.Popen(args + ["daemon"], stdout=daemon_log, stderr=daemon_log)
        fixture = None
        ready = None
        def command(*parts):
            result = subprocess.run(args + list(parts), capture_output=True, text=True, timeout=5)
            assert result.returncode == 0, result.stderr
            return json.loads(result.stdout)
        def events():
            return [json.loads(line) for line in (directory / "events.jsonl").read_text().splitlines()]
        try:
            deadline = time.monotonic() + 5
            while True:
                try:
                    command("status")
                    break
                except AssertionError:
                    assert daemon.poll() is None and time.monotonic() < deadline
                    time.sleep(0.05)
            fixture = subprocess.Popen([str(fixture_binary)], stdout=event_log)
            while ready is None:
                ready = next((event for event in events() if event.get("ready")), None)
                assert fixture.poll() is None and time.monotonic() < deadline + 3
                time.sleep(0.05)
            assert ready["visible"] and ready["key"] and ready["frontmost"] == ready["pid"], ("test window is not safely focused", ready)
            command("move-to", "--x", str(ready["x"]), "--y", str(ready["y"]))
            time.sleep(0.2)
            actual = command("status")["position"]
            assert abs(actual["x"] - ready["x"]) < 2 and abs(actual["y"] - ready["y"]) < 2, ("cursor did not move", actual, ready)
            command("click", "--count", "2")
            command("click", "--button", "right", "--modifiers", "cmd")
            command("click", "--button", "middle")
            command("button-down")
            command("move", "--dx", "20")
            command("button-up")
            command("scroll", "--dx", "20", "--dy", "30")
            time.sleep(0.2)
            received = events()
            assert any(e.get("event") == "left-down" and e["count"] == 2 for e in received), received
            assert any(e.get("event") == "right-down" and e["command"] for e in received), received
            for kind in ["left-up", "right-up", "middle-down", "middle-up", "drag"]:
                assert any(e.get("event") == kind for e in received), (kind, received)
            assert any(e.get("event") == "scroll" and e["dx"] != 0 and e["dy"] != 0 for e in received), received
            config.write_text("[keybindings]\nnavigation_enabled = true\n[system]\nhot_reload = false\n")
            command("reload")
            command("activate", "freestyle")
            before = command("status")["position"]
            native = ctypes.CDLL("/System/Library/Frameworks/ApplicationServices.framework/ApplicationServices")
            native.CGEventCreateKeyboardEvent.argtypes = [ctypes.c_void_p, ctypes.c_uint16, ctypes.c_bool]
            native.CGEventCreateKeyboardEvent.restype = ctypes.c_void_p
            native.CGEventPost.argtypes = [ctypes.c_uint32, ctypes.c_void_p]
            native.CFRelease.argtypes = [ctypes.c_void_p]
            def right_arrow():
                for pressed in [True, False]:
                    event = native.CGEventCreateKeyboardEvent(None, 124, pressed)
                    assert event
                    native.CGEventPost(0, event)
                    native.CFRelease(event)
                time.sleep(0.15)
            right_arrow()
            after = command("status")["position"]
            assert abs(after["x"] - before["x"] - 10) < 2, ("active arrow did not move", before, after)
            assert not any(e.get("event") == "key" for e in events()), "active arrow leaked to the app"
            command("deactivate")
            right_arrow()
            assert any(e.get("event") == "key" and e["code"] == 124 for e in events()), "inactive arrow was swallowed"

        finally:
            try:
                if daemon.poll() is None:
                    try:
                        command("stop")
                        if ready:
                            command("move-to", "--x", str(ready["original_x"]), "--y", str(ready["original_y"]))
                        command("quit")
                        daemon.wait(timeout=5)
                    finally:
                        if daemon.poll() is None:
                            daemon.terminate()
                            try:
                                daemon.wait(timeout=5)
                            except subprocess.TimeoutExpired:
                                daemon.kill()
                                daemon.wait()
            finally:
                if fixture is not None and fixture.poll() is None:
                    fixture.terminate()
                    try:
                        fixture.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        fixture.kill()
                        fixture.wait()

print("Passed: real mouse actions, active keyboard suppression, and inactive key passthrough.")
