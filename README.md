<div align="center">
  <h1 align="center">
    <a href="https://github.com/iamgideonidoko/kact">
      <img src="https://raw.githubusercontent.com/iamgideonidoko/kact/main/docs/public/kact.svg" alt="Kact" width="160" height="160" />
      <br />
      Kact
    </a>
  </h1>

  <p>
    <a href="https://github.com/iamgideonidoko/kact/releases"><img alt="GitHub Release" src="https://img.shields.io/github/v/release/iamgideonidoko/kact?display_name=tag&style=for-the-badge" /></a>
    <a href="https://github.com/iamgideonidoko/kact/blob/main/LICENSE"><img alt="License" src="https://img.shields.io/github/license/iamgideonidoko/kact?style=for-the-badge" /></a>
    <a href="https://github.com/iamgideonidoko/kact/actions/workflows/check.yml"><img alt="Checks" src="https://img.shields.io/github/actions/workflow/status/iamgideonidoko/kact/check.yml?branch=main&style=for-the-badge&label=checks" /></a>
  </p>

  <p><b>Keyboard-driven cursor ACTuator</b></p>

  <p align="center">
    <a href="https://iamgideonidoko.github.io/kact/getting-started/installation/">Installation</a> •
    <a href="https://iamgideonidoko.github.io/kact/">Documentation</a> •
    <a href="https://iamgideonidoko.github.io/kact/configuration/overview/">Configuration</a> •
    <a href="https://iamgideonidoko.github.io/kact/reference/cli/">CLI</a> •
    <a href="CONTRIBUTING.md">Contributing</a>
  </p>

  <hr />
</div>

Kact is a command-first, keyboard-driven cursor actuator for macOS and Linux X11. It provides precise movement, clicks, dragging, scrolling, jumps, and optional labeled target navigation. macOS adds native overlays, Accessibility targeting, and opt-in keyboard capture; Linux X11 focuses on shell/remapper-driven pointer control. Global shortcuts stay off by default.

## Install

Published binaries are installed from GitHub Releases:

```sh
curl --proto '=https' --tlsv1.2 -fsSL https://raw.githubusercontent.com/iamgideonidoko/kact/main/scripts/install.sh | bash # latest release
kact setup
```

The command installs the latest release. To select a specific tag, use `bash -s -- --version vX.Y.Z` after the pipe. The installer selects the macOS Apple Silicon, macOS Intel, or Linux x86_64 archive, verifies its SHA-256 checksum before installing, writes the executable to `~/.local/bin` by default, and records its installation receipt under `~/.local/state/kact`. Current macOS releases are unsigned and unnotarized, so macOS may ask for confirmation before the first run. See the [installation guide](https://iamgideonidoko.github.io/kact/getting-started/installation/) for source builds, updates, and supported platforms.

## First action

```sh
kact status
```

## Build from source

Install the pinned Rust 1.88.0 toolchain:

```sh
mise install
cargo install --locked --path .
kact setup
kact status
```

`kact setup` creates the default configuration, opens macOS Accessibility settings when needed, and starts the daemon. After granting permission, rerun `kact setup`. If keyboard capture is denied, check Input Monitoring too. `kact doctor` checks configuration, permission, and service availability without moving the cursor.

## Roadmap

- Harden macOS pointer, permission, display, remapper, and long-running-session workflows; publish latency and resource measurements.
- Add Developer ID signing and notarization for macOS releases.
- Add configuration presets, window-relative movement and glides, and held scrolling.
- Harden X11 across desktop environments and report usable capabilities through `kact doctor`.
- Add explicit Wayland profiles, beginning with supported wlroots compositors.
- Add a native Windows backend: pointer actions first, then keyboard capture, overlays, accessibility targets, startup, and signed releases.
- Add per-app profiles, macros, and a graphical configuration editor.

## License

[MIT](LICENSE).
