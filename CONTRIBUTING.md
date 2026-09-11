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

Run `make coverage-core` after changing `src/core/`. Run `make smoke` and `make smoke-mouse` after changing macOS overlays, accessibility, keyboard capture, or pointer injection. Run `pnpm --dir docs run check` and `make docs-build` when changing the documentation site. Update the README, relevant `docs/src/content/docs/` page, and `CHANGELOG.md` when user-facing behavior changes.

Keep changes focused, preserve command-first operation, and include tests for behavior that can regress.

Please follow the [Code of Conduct](CODE_OF_CONDUCT.md). For setup and usage help, see [SUPPORT.md](SUPPORT.md); use GitHub issues for reproducible bugs and concrete feature requests.
