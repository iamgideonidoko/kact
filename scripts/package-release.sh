#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/package-release.sh [--target RUST_TARGET] [--output-dir DIRECTORY]

Build one release archive and its SHA-256 sidecar.
EOF
}

target=""
output_dir="dist"
while (($#)); do
  case "$1" in
    --target) target=${2:?missing target}; shift 2 ;;
    --output-dir) output_dir=${2:?missing output directory}; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) printf 'Unknown option: %s\n' "$1" >&2; usage >&2; exit 2 ;;
  esac
done

if [[ -z "$target" ]]; then
  target=$(rustc -vV | sed -n 's/^host: //p')
fi

case "$target" in
  aarch64-apple-darwin) platform=macos-arm64 ;;
  x86_64-apple-darwin) platform=macos-x86_64 ;;
  x86_64-unknown-linux-gnu) platform=linux-x86_64 ;;
  *) printf 'Unsupported release target: %s\n' "$target" >&2; exit 1 ;;
esac

version=$(awk '
  /^\[package\]$/ { in_package = 1; next }
  in_package && /^\[/ { exit }
  in_package && /^version = / { gsub(/"/, "", $3); print $3; exit }
' Cargo.toml)
[[ -n "$version" ]] || { printf 'Could not read package version from Cargo.toml\n' >&2; exit 1; }

cargo build --locked --release --target "$target"

archive="kact-${platform}.tar.gz"
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
package_dir="kact-v${version}-${platform}"
mkdir -p "$stage/$package_dir" "$output_dir"
install -m 755 "target/$target/release/kact" "$stage/$package_dir/kact"
install -m 644 README.md LICENSE "$stage/$package_dir"
printf '%s\n' "$version" > "$stage/$package_dir/VERSION"

python3 scripts/create_archive.py "$stage/$package_dir" "$output_dir/$archive"
if command -v shasum >/dev/null 2>&1; then
  shasum -a 256 "$output_dir/$archive" | awk -v name="$archive" '{ print $1 "  " name }' > "$output_dir/$archive.sha256"
else
  sha256sum "$output_dir/$archive" | awk -v name="$archive" '{ print $1 "  " name }' > "$output_dir/$archive.sha256"
fi
printf 'Created %s\n' "$output_dir/$archive"
