---
title: Testing
description: Automated checks and macOS native smoke checks.
---

```sh
make lint
make test
make release
```

After changing `src/core/`, also run:

```sh
make coverage-core
```

`make smoke` checks macOS daemon, overlays, config reload, and shutdown without moving pointer. `make smoke-mouse` opens temporary test window to exercise real pointer actions and keyboard capture, then restores pointer. `make smoke-elements` opens a native fixture with standard controls, a long clipped document, a nested viewport, an inaccessible canvas surface, and a later relayout. It validates anonymous Accessibility structure, visible-only bounds, bounded discovery, and a fresh post-layout snapshot through `inspect elements --pid`.

Install the local reporter once with:

```sh
mise exec rust@1.88.0 -- cargo install cargo-llvm-cov --locked
```

`make coverage` opens an HTML report. `make coverage-core` checks line coverage for `src/core/`; CI requires 100% line coverage.

Run all three smoke checks after changing overlays, accessibility, input capture, pointer injection, or runtime code that connects them. They require macOS desktop, Accessibility permission, and Xcode command-line tools. Fixture check is deterministic native coverage, not substitute for manual compatibility checks in browsers, Electron, app-provided virtualized lists, or canvas interfaces.

CI builds and tests on macOS and Linux. It does not replace manual desktop verification of the X11 backend.

The Docs workflow builds the site for documentation edits and changes to source, release scripts, build metadata, `README.md`, or Makefile commands. It catches site build regressions when a documented interface changes; review text accuracy with the interface change itself.
