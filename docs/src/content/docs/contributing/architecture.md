---
title: Architecture
description: Module responsibilities and platform boundaries.
---

| Area | Responsibility |
| --- | --- |
| `src/command.rs` | Clap CLI and serializable command model. |
| `src/config.rs` | TOML defaults, loading, and validation. |
| `src/ipc.rs` | Private Unix socket, framing, and single-instance locking. |
| `src/core/` | Motion, geometry, labels, and input state without platform APIs. |
| `src/runtime/` | Command execution, bindings, lifecycle, and config watching. |
| `src/platform/` | macOS/X11 pointer injection and macOS keyboard input. |
| `src/desktop/` | macOS AppKit overlay and accessibility discovery. |
| `src/service.rs` | macOS LaunchAgent setup. |

Keep shared semantics in the core/runtime layers. Platform modules implement operating-system mechanics without changing command behavior. Native macOS UI stays on the main thread.

See [architecture decisions](/contributing/decisions/) for the command-first decision that defines the CLI as the stable integration boundary.
