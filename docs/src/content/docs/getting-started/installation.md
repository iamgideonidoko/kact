---
title: Installation
description: Install a verified GitHub Release or build Kact from source.
---

Kact ships release archives through GitHub Releases. The installer downloads the matching archive, verifies its SHA-256 checksum, then installs only the `kact` executable.

## Requirements

- macOS or a Linux X11 session.
- Linux releases support x86_64 X11 sessions. Native Wayland is unsupported.
- Current macOS releases are unsigned and unnotarized. macOS may ask you to confirm before the first run.

## Install a release

The latest release installs with:

```sh
curl --proto '=https' --tlsv1.2 -fsSL https://raw.githubusercontent.com/iamgideonidoko/kact/main/scripts/install.sh | bash # latest release
kact --version
```

The command installs the latest release. To select a specific tag, pass its version through `bash -s --`:

```sh
curl --proto '=https' --tlsv1.2 -fsSL https://raw.githubusercontent.com/iamgideonidoko/kact/main/scripts/install.sh | bash -s -- --version vX.Y.Z
```

The default destination is `~/.local/bin`. Choose a different one with `KACT_INSTALL_DIR`:

```sh
curl --proto '=https' --tlsv1.2 -fsSL https://raw.githubusercontent.com/iamgideonidoko/kact/main/scripts/install.sh | KACT_INSTALL_DIR="$HOME/bin" bash
```

Ensure the destination is on `PATH`, then complete setup:

```sh
kact setup
```

To update a release-script installation, run:

```sh
kact update
```

Kact finds the latest stable GitHub Release, verifies the matching archive against its published SHA-256 checksum, atomically replaces its executable, and restarts a running daemon. Check without changing anything with `kact update --check`.

`kact update` accepts only an unmodified binary installed by the release script. Update Cargo, Homebrew, and source-checkout installations with their owning tool instead. You can still rerun the installer when you need a specific release tag.

## Uninstall

```sh
kact uninstall
```

This stops Kact, removes its managed macOS login service, and removes a binary installed by the release script. Configuration and logs stay in place. Use `kact uninstall --purge` to remove the active configuration, log, and socket too.

Kact verifies its release-install receipt before deleting a binary. Cargo and Homebrew installations are left alone; remove those with the tool that installed them.

## Build from source

Source builds need Rust 1.88.0 with `rustfmt` and `clippy`. The repository pins both in `rust-toolchain.toml` and `.mise.toml`. Linux source builds also need X11 and XTest development headers.

Install the Linux dependencies before building:

```sh
# Debian and Ubuntu
sudo apt-get install libx11-dev libxtst-dev

# Arch Linux and Omarchy
sudo pacman -S libx11 libxtst
```

## Install with Mise

```sh
git clone https://github.com/iamgideonidoko/kact.git
cd kact
mise install
cargo install --locked --path .
kact --version
```

## Install with Rustup

```sh
rustup toolchain install 1.88.0 --profile minimal --component rustfmt --component clippy
git clone https://github.com/iamgideonidoko/kact.git
cd kact
cargo install --locked --path .
kact --version
```

`cargo install --path .` installs the executable into Cargo’s binary directory, commonly `~/.cargo/bin`. Ensure that directory is on `PATH` before using Kact from a GUI shortcut tool.

## Update after pulling changes

```sh
git pull
mise install
cargo install --locked --path . --force
```

Restart the daemon after installing an update:

```sh
kact quit
kact setup
```
