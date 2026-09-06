#!/usr/bin/env python3
"""Build a local menu-bar app. Set KACT_SIGN_IDENTITY for Developer ID signing."""
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import sys
import tomllib

if sys.platform != "darwin":
    raise SystemExit("App bundles must be built on macOS.")
root = Path(__file__).resolve().parents[1]
subprocess.run(["cargo", "build", "--release", "--locked"], cwd=root, check=True)
package = tomllib.loads((root / "Cargo.toml").read_text())["package"]
bundle = root / "target" / "Kact.app"
macos = bundle / "Contents" / "MacOS"
resources = bundle / "Contents" / "Resources"
macos.mkdir(parents=True, exist_ok=True)
resources.mkdir(parents=True, exist_ok=True)
shutil.copy2(root / "target" / "release" / "kact", macos / "kact")
shutil.copy2(root / "README.md", resources / "README.md")
shutil.copy2(root / "LICENSE", resources / "LICENSE")
with (bundle / "Contents" / "Info.plist").open("wb") as output:
    plistlib.dump({
        "CFBundleIdentifier": "io.kact.app",
        "CFBundleName": "Kact",
        "CFBundleDisplayName": "Kact",
        "CFBundleExecutable": "kact",
        "CFBundlePackageType": "APPL",
        "CFBundleShortVersionString": package["version"],
        "CFBundleVersion": package["version"],
        "LSUIElement": True,
        "LSMinimumSystemVersion": "11.0",
        "NSHighResolutionCapable": True,
    }, output)
identity = os.environ.get("KACT_SIGN_IDENTITY", "-")
args = ["codesign", "--force", "--sign", identity]
if identity != "-":
    args += ["--options", "runtime", "--timestamp"]
subprocess.run(args + [str(bundle)], check=True)
subprocess.run(["codesign", "--verify", "--strict", str(bundle)], check=True)
print(f"Built {bundle}. Move it to ~/Applications or /Applications, then open it.")
print("For shell commands, use Kact.app/Contents/MacOS/kact or add a symlink on PATH.")
