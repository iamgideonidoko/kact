#!/usr/bin/env bash
set -euo pipefail

repository=iamgideonidoko/kact
install_dir=${KACT_INSTALL_DIR:-"$HOME/.local/bin"}
state_home=${XDG_STATE_HOME:-}
[[ "$state_home" == /* ]] || state_home="$HOME/.local/state"
state_dir="$state_home/kact"
version=""

usage() {
  cat <<'EOF'
Usage: install.sh [--version vX.Y.Z] [--install-dir DIRECTORY]

Installs Kact from a GitHub Release. The latest release is used unless
--version is supplied. Set KACT_INSTALL_DIR to change the default destination.
EOF
}

while (($#)); do
  case "$1" in
    --version) version=${2:?missing version}; shift 2 ;;
    --install-dir) install_dir=${2:?missing directory}; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) printf 'Unknown option: %s\n' "$1" >&2; usage >&2; exit 2 ;;
  esac
done

case "$(uname -s)" in
  Darwin)
    case "$(uname -m)" in
      arm64) platform=macos-arm64 ;;
      x86_64) platform=macos-x86_64 ;;
      *) printf 'Unsupported macOS architecture: %s\n' "$(uname -m)" >&2; exit 1 ;;
    esac
    ;;
  Linux)
    [[ -z ${WAYLAND_DISPLAY:-} ]] || {
      printf 'Native Wayland is unsupported. Run Kact in an X11 session.\n' >&2
      exit 1
    }
    [[ $(uname -m) == x86_64 ]] || {
      printf 'Unsupported Linux architecture: %s\n' "$(uname -m)" >&2
      exit 1
    }
    platform=linux-x86_64
    ;;
  *) printf 'Unsupported operating system: %s\n' "$(uname -s)" >&2; exit 1 ;;
esac

if [[ -n "$version" && ! "$version" =~ ^v[0-9]+\.[0-9]+\.[0-9]+([-.+][0-9A-Za-z.-]+)?$ ]]; then
  printf 'Version must look like vX.Y.Z: %s\n' "$version" >&2
  exit 2
fi

asset="kact-${platform}.tar.gz"
if [[ -n "$version" ]]; then
  release_url="https://github.com/$repository/releases/download/$version"
else
  release_url="https://github.com/$repository/releases/latest/download"
fi

workdir=$(mktemp -d)
trap 'rm -rf "$workdir"' EXIT
curl --fail --location --silent --show-error "$release_url/SHA256SUMS" -o "$workdir/SHA256SUMS"
curl --fail --location --silent --show-error "$release_url/$asset" -o "$workdir/$asset"

expected=$(awk -v asset="$asset" '$2 == asset { print $1; exit }' "$workdir/SHA256SUMS")
[[ -n "$expected" ]] || { printf 'No checksum for %s in SHA256SUMS\n' "$asset" >&2; exit 1; }
if command -v shasum >/dev/null 2>&1; then
  actual=$(shasum -a 256 "$workdir/$asset" | awk '{ print $1 }')
else
  actual=$(sha256sum "$workdir/$asset" | awk '{ print $1 }')
fi
[[ "$actual" == "$expected" ]] || { printf 'Checksum verification failed for %s\n' "$asset" >&2; exit 1; }

mkdir "$workdir/extract"
tar -xzf "$workdir/$asset" -C "$workdir/extract"
binary=$(find "$workdir/extract" -type f -name kact -perm -u+x -print -quit)
[[ -n "$binary" ]] || { printf 'Release archive does not contain an executable kact binary\n' >&2; exit 1; }

[[ "$install_dir" != *$'\n'* && "$install_dir" != *$'\r'* ]] || {
  printf 'Install directory cannot contain a newline\n' >&2
  exit 2
}
mkdir -p "$install_dir"
install_dir=$(cd "$install_dir" && pwd -P)
temporary="$install_dir/.kact.$$"
install -m 755 "$binary" "$temporary"
mv -f "$temporary" "$install_dir/kact"
if [[ $(uname -s) == Darwin ]]; then
  identity=$(stat -f '%d %i' "$install_dir/kact")
else
  identity=$(stat -c '%d %i' "$install_dir/kact")
fi
read -r device inode <<< "$identity"
mkdir -p "$state_dir"
receipt="$state_dir/install-receipt"
temporary_receipt="$receipt.$$"
printf 'path=%s\ndevice=%s\ninode=%s\n' "$install_dir/kact" "$device" "$inode" > "$temporary_receipt"
chmod 600 "$temporary_receipt"
mv -f "$temporary_receipt" "$receipt"
printf 'Installed kact to %s\n' "$install_dir/kact"
"$install_dir/kact" --version
case ":$PATH:" in
  *":$install_dir:"*) printf 'Next: kact setup\n' ;;
  *) printf 'Add %s to PATH, then run: kact setup\n' "$install_dir" ;;
esac
