#!/usr/bin/env python3
"""Fail when line coverage for the pure core falls below its baseline."""

import json
import sys
from pathlib import Path

MINIMUM = 100.0
CORE = Path("src/core")


def main() -> None:
    report = json.loads(Path(sys.argv[1]).read_text())
    files = report["data"][0]["files"]
    core = [file for file in files if Path(file["filename"]).resolve().is_relative_to(CORE.resolve())]
    if not core:
        raise SystemExit("coverage report contains no core files")
    covered = sum(file["summary"]["lines"]["covered"] for file in core)
    total = sum(file["summary"]["lines"]["count"] for file in core)
    percentage = covered / total * 100 if total else 100
    print(f"Core line coverage: {covered}/{total} ({percentage:.2f}%)")
    if percentage < MINIMUM:
        raise SystemExit(f"core coverage must be at least {MINIMUM:.2f}%")


if __name__ == "__main__":
    main()
