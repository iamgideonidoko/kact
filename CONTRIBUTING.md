# Contributing to Kact

## Setup

Install Rust 1.88.0 with Mise:

```sh
mise install
make build
make test
```

With Rustup instead, install the same toolchain and components:

```sh
rustup toolchain install 1.88.0 --profile minimal --component rustfmt --component clippy
```

Linux X11 builds require `libx11-dev` and `libxtst-dev`. macOS native checks require Accessibility permission; mouse smoke checks also require Xcode command-line tools.

## Before opening a pull request

```sh
make lint
make test
make release
```

Run `make smoke` and `make smoke-mouse` after changing macOS overlays, accessibility, keyboard capture, or pointer injection. Update the README, relevant documentation, and `CHANGELOG.md` when user-facing behavior changes.

Keep changes focused, preserve command-first operation, and include tests for behavior that can regress.
