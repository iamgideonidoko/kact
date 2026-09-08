#!/usr/bin/env bash
set -euo pipefail

version=${1:?Usage: scripts/release-notes.sh VERSION}
awk -v heading="## ${version}" '
  $0 == heading { found = 1; next }
  found && /^## / { exit }
  found { print }
  END { if (!found) exit 1 }
' CHANGELOG.md || {
  printf 'CHANGELOG.md has no section for %s\n' "$version" >&2
  exit 1
}
