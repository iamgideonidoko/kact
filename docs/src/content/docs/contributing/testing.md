---
title: Testing
description: Automated checks and macOS native smoke checks.
---

```sh
make lint
make test
make release
```

`make smoke` checks the macOS daemon, overlays, config reload, and shutdown without moving the pointer. `make smoke-mouse` opens a temporary test window to exercise real pointer actions and keyboard capture, then restores the pointer.

Install the local reporter once with:

```sh
mise exec rust@1.88.0 -- cargo install cargo-llvm-cov --locked
```

`make coverage` opens an HTML report. `make coverage-core` checks line coverage for `src/core/`; CI requires 100% line coverage.

Run both smoke checks after changing overlays, accessibility, input capture, pointer injection, or the runtime code that connects them. They require a macOS desktop and Accessibility permission; `smoke-mouse` also needs Xcode command-line tools.

CI builds and tests on macOS and Linux. It does not replace manual desktop verification of the X11 backend.
