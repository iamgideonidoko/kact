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

## Test backlog

This is the complete behavior-based backlog for feasible Kact tests. It is not an instruction to chase blanket line coverage: native wrappers need behavior checks against real platforms, while the pure `src/core/` code remains at 100% line coverage.

### Release, installation, and removal

- Update metadata: unavailable or failing `curl`, malformed JSON, missing fields, draft and prerelease responses, stable-tag variants, and safe error reporting.
- Update installation: checksum/archive download failures; malformed, duplicate, missing, or wrong-asset checksum records; corrupt or invalid archives; missing, non-executable, or wrong-version binaries; unsupported OS/architecture; temporary-directory collisions and cleanup.
- Atomic binary replacement: source read, write, sync, rename, and temporary-path collision failures must leave the installed binary intact.
- Installation receipts: valid current-executable/device/inode match; missing, malformed, duplicated, relative, foreign-owned, writable, linked, unreadable, or stale receipts; `XDG_STATE_HOME` and `HOME` fallbacks; atomic receipt refresh; managed-only removal.
- Uninstall: managed and unmanaged installations; `--purge`; absent configuration, logs, or socket; deletion failures.
- Release scripts: local fake release server or fake `curl` for successful install, checksum mismatch, bad archive, unsafe paths, failed file operations, all platform/version argument paths, archive contents, executable modes, reproducibility, and checksum sidecars.

### Daemon and IPC transport

- Daemon startup: missing config creation, duplicate daemon, existing service, child exit, readiness timeout, invalid config/log level/socket, permission denial, and log-level forwarding.
- Daemon exit: `quit`, `SIGINT`, `SIGTERM`, runtime failure, held-button cleanup, listener shutdown, and socket removal.
- Request handling: queued-request expiration, command errors without daemon death, graceful restart after update, and response serialization.
- Socket safety: missing/non-directory/symlink/wrong-owner/wrong-mode parents; lock-file symlink, hardlink, wrong mode, foreign owner, and contention; foreign/live/stale sockets; replacement race during cleanup.
- Framing: empty, whitespace, truncated, multiple, oversized, invalid UTF-8, valid-plus-junk, exact size boundary, interrupted reads, read/write timeout, queue full, client disconnect, and shutdown while reply waits.
- Command envelopes: unknown fields and invalid values for every command shape, including unit variants.
- Default socket path: valid, invalid, and missing `XDG_RUNTIME_DIR`.

### Runtime behavior

- Status output with inactive, active, moving, gliding, and unavailable-pointer states.
- Grid, Elements, and Freestyle activation; toggle/deactivate idempotency; discovery/display failures; empty displays; drop cleanup.
- Move, move-to, zero/interrupted/completed glides, every mouse button/count/modifier, drag, multi-button release, scrolling, and cursor errors.
- Continuous movement: every direction/speed, repeated starts, competing speed overrides, stop behavior, frame timing, tick caps, and movement errors.
- Navigation: cancel, backspace, invalid/incomplete labels, refinement, nested refinement, auto-click semantic/fallback behavior, selection during refresh, and focused-window changes.
- Elements: filtering, cycling/wrap, empty results, direct refresh, stale notifications, changed/unchanged targets, retries, settle probes, and display/focus changes.
- Presentation: each `show` setting, min/max bounds, grid rebuild, and no-grid behavior.
- Jump: every target, cycle order, negative display origins, unmatched cursor, and no screens.
- Keyboard: global/local binding precedence, key-up direction release, repeat policy, label typing, input listener errors, and poll event cap.
- Configuration reload: valid replacement, invalid replacement preservation, listener replacement/restore failure, disabled hot reload, and reload while gliding, dragging, navigating, filtering, or moving.

### Configuration, CLI, and bindings

- TOML valid boundaries, invalid neighbors, unknown fields at every level, wrong field types, finite-number handling, colors, presets, log levels, easing, and label positions.
- Grid/Elements override inheritance for every setting and alphabet de-duplication/order.
- Config paths: absolute/relative/missing `XDG_CONFIG_HOME`, missing `HOME`, unreadable/missing/non-UTF-8 config files.
- Clap and serde acceptance/rejection for every command, enum variant, option default, option bound, and global option.
- Binding parser whitespace, aliases, duplicate/unknown modifiers, shell-looking strings, and forbidden service/setup/config actions.
- Shortcut normalization aliases, key forms, function keys, malformed shortcuts, disabled-but-invalid bindings, duplicate canonical bindings, Vi/Emacs maps, and label alphabets exhausted by local bindings.
- CLI paths: help; config commands; doctor; setup; start; daemon; service; update; uninstall; and inspect baseline save/diff errors.

### Configuration watcher and login service

- Watcher relative paths, absent paths, direct writes, delete/recreate, atomic rename, rapid event coalescing, invalid replacement, newest-valid-wins, symlinked parents, and watcher/thread failure.
- Login service behavior: non-macOS error; missing `HOME`; managed/unmanaged/non-file/symlink plist; XML escaping/control characters; launch arguments; file mode/no-follow behavior; launchctl success/failure; idempotent uninstall.

### Linux X11

Run these under Xvfb in Linux CI.

- Missing display, Wayland rejection, X11/XTest initialization failure.
- Fractional relative-motion remainder, absolute-motion reset, non-finite coordinates, pointer-query failure.
- Left/middle/right mapping, click count, modifier rejection, duplicate button holds, release behavior, scroll signs and limits, `release_all`, and Drop cleanup.
- End-to-end pointer position, button, drag, and scroll events observed by an X11 fixture.

### macOS native behavior

Run these on a self-hosted macOS desktop runner with Accessibility permission.

- Event-tap permission, creation, timeout/re-enable, key down/up/repeat, modifiers, layout translation, global/navigation/label capture rules, wheel pass-through, overflow, and stop/drop.
- Cursor position/movement errors; all click/drag/modifier/scroll paths; multi-button cleanup.
- Overlay appearance, label positions, repeated show/hide, AppKit cleanup, multi-display scale/origin/reconnect, overlapping displays, tiny targets, and focus/pointer pass-through.
- Accessibility permission, missing AX values/actions/children, AX timeouts, deep/wide trees, hidden/disabled/offscreen/clipped targets, nested scroll views, virtualized lists, modal/sheet/popover/titlebar/minimized/fullscreen windows, target ranking, filtering, element press, observer storms, focus changes during scan, and relayout during scan.
- OCR fallback: Screen Recording denied/granted, warmup failure, stable/jitter snapshots, and semantic/visual target merging.
- Add native fixtures for multi-display, modal, denied accessibility, skeletal browser/Electron trees, virtualized lists, OCR-only canvases, and failed element press.

### Documentation and robustness

- Run `pnpm --dir docs run check` in CI as well as site build.
- Syntax-check shell scripts with `bash -n` and Python scripts with `python3 -m py_compile`.
- Check README/docs CLI and configuration snippets against `--help` and default `kact.toml`.
- Verify release-workflow package artifacts and checksums before publishing.
- Add deterministic property tests for shortcut normalization, command JSON, config TOML, labels/navigation invariants, motion/glide finiteness, and IPC framing.

## Automation boundary

Most backlog items are fully automatable: Rust unit/integration tests, subprocess tests, temporary directories, fake command binaries, fake release endpoints, Xvfb, and docs/static checks.

macOS native smoke tests are also automatable, but only on a desktop runner with persistent Accessibility permission and, for OCR, Screen Recording permission. They cannot run reliably on ordinary hosted CI machines.

Manual compatibility checks remain necessary for third-party applications whose behavior cannot be reproduced deterministically: real browsers, Electron applications, app-specific virtualized lists, custom canvas interfaces, user keyboard layouts, display hardware, and assistive-technology interactions. Manual checks complement automated fixtures; they are not a substitute for them.
