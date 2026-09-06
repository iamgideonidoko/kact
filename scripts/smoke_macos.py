#!/usr/bin/env python3
"""Exercise the real macOS service and overlays without moving or clicking the mouse."""
import json
import os
from pathlib import Path
import signal
import socket
import subprocess
import sys
import tempfile
import time

binary = Path(sys.argv[1] if len(sys.argv) > 1 else "target/debug/kact").resolve()
if sys.platform != "darwin":
    raise SystemExit("This check requires a macOS desktop and Accessibility permission.")
with tempfile.TemporaryDirectory(prefix="kact-smoke-") as directory:
    directory = Path(directory)
    config = directory / "config.toml"
    control = directory / "private" / "control.sock"
    config.write_text("[keybindings]\nnavigation_enabled = false\n")
    args = [str(binary), "--config", str(config), "--socket", str(control)]

    def run(*command, success=True):
        result = subprocess.run(args + list(command), capture_output=True, text=True, timeout=8)
        assert (result.returncode == 0) == success, (command, result.stdout, result.stderr)
        return json.loads(result.stdout) if result.returncode == 0 else result.stderr

    def ready(process):
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            assert process.poll() is None, "service exited before becoming ready"
            if control.exists():
                try:
                    return run("status")
                except (AssertionError, subprocess.TimeoutExpired):
                    pass
            time.sleep(0.05)
        raise AssertionError("service never became ready")

    with (directory / "service.log").open("w+") as log:
        process = subprocess.Popen(args + ["daemon"], stdout=log, stderr=log)
        try:
            assert ready(process)["active"] is False
            run("activate", "grid")
            state = run("status")
            assert state["mode"] == "grid" and state["targets"] > 0
            run("select", "a")
            assert run("status")["prefix"] == "a"
            run("cancel")
            assert run("status")["active"] is True
            run("cancel")
            assert run("status")["active"] is False
            run("activate", "elements")
            assert run("status")["targets"] > 0
            run("deactivate")
            run("select", "aa", success=False)
            config.write_text("[motion]\ntarget_fps = 0\n")
            run("reload", success=False)
            assert run("status")["running"] is True
            replacement = directory / "replacement.toml"
            replacement.write_text("[motion]\ntarget_fps = 60\n[keybindings]\nnavigation_enabled = true\n")
            os.replace(replacement, config)
            deadline = time.monotonic() + 3
            while not run("status")["navigation_keyboard"]:
                assert time.monotonic() < deadline, "atomic config reload failed"
                time.sleep(0.05)
            # Starts/stops the native event tap but does not inject any keyboard event.
            run("activate", "freestyle")
            run("deactivate")
            with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
                client.settimeout(2)
                client.connect(str(control))
                client.sendall(b'{"action":"stop","unexpected":true}\n')
                assert json.loads(client.makefile().readline())["ok"] is False
            run("quit")
            assert process.wait(timeout=5) == 0
            assert not control.exists()
            process = subprocess.Popen(args + ["daemon"], stdout=log, stderr=log)
            ready(process)
            process.send_signal(signal.SIGTERM)
            assert process.wait(timeout=5) == 0
            assert not control.exists()
        except BaseException:
            log.flush()
            print((directory / "service.log").read_text(), file=sys.stderr)
            raise
        finally:
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
print("Passed: service, grid/elements, prefix/cancel, native capture, atomic reload, invalid requests, quit, SIGTERM.")
