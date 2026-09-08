#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/publish-release.sh

Publish a prepared release from main. Before running, set Cargo.toml's package
version and add a matching CHANGELOG.md section. Only Cargo.toml, Cargo.lock,
and CHANGELOG.md may be changed; the script verifies, commits, tags, and pushes.
EOF
}

[[ ${1:-} != --help && ${1:-} != -h ]] || { usage; exit 0; }
[[ $# -eq 0 ]] || { usage >&2; exit 2; }

die() { printf '%s\n' "$*" >&2; exit 1; }
version=$(awk '
  /^\[package\]$/ { in_package = 1; next }
  in_package && /^\[/ { exit }
  in_package && /^version = / { gsub(/"/, "", $3); print $3; exit }
' Cargo.toml)
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+([-.+][0-9A-Za-z.-]+)?$ ]] || {
  die "Cargo.toml version must look like X.Y.Z or X.Y.Z-rc.N: $version"
}

assert_release_changes() {
  while IFS= read -r change; do
    [[ -z "$change" ]] && continue
    path=${change:3}
    case "$path" in
      Cargo.toml|Cargo.lock|CHANGELOG.md) ;;
      *) die "Release preparation may only change Cargo.toml, Cargo.lock, and CHANGELOG.md; found $path." ;;
    esac
  done < <(git status --porcelain --untracked-files=all)
}

[[ $(git branch --show-current) == main ]] || die 'Releases must be published from main.'
git remote get-url origin >/dev/null || die 'The origin remote is required.'
[[ -z $(git tag --list "v$version") ]] || die "Tag v$version already exists."

notes=$(scripts/release-notes.sh "$version")
[[ -n ${notes//[[:space:]]/} ]] || die "CHANGELOG.md section $version must contain release notes."

assert_release_changes

make lint
make test
make docs-build
make package
assert_release_changes
git diff --check
git diff --cached --check

if [[ -n $(git status --porcelain -- Cargo.toml Cargo.lock CHANGELOG.md) ]]; then
  git add Cargo.toml Cargo.lock CHANGELOG.md
  git commit -m "chore: release v$version"
fi
git tag -a "v$version" -m "v$version"
git push origin main --follow-tags
printf 'Published v%s. GitHub Actions will create the GitHub Release.\n' "$version"
